use std::cell::Ref;

use candid::Principal;
use ic_cdk_macros::{query, update};

use super::{domain::*, error::UserError};

use crate::common::guard::user_owner_guard;
use crate::context::DaoContext;
use crate::CONTEXT;

#[update]
fn register_user(cmd: UserRegisterCommand) -> Result<String, UserError> {
    CONTEXT.with(|c| {
        let mut ctx = c.borrow_mut();
        let id = ctx.id;
        let caller = ctx.env.caller();

        if caller == Principal::anonymous() {
            return Err(UserError::AnonymousNotAllowRegistering);
        }

        let now = ctx.env.now();
        let user = cmd.build_profile(id, caller, UserStatus::Enable, now);

        match ctx.user_service.insert_user(user) {
            Ok(p) => {
                ctx.id += 1; // 注册成功，id + 1
                Ok(p.to_string())
            }
            Err(e) => Err(e),
        }
    })
}

#[update]
fn auto_register_user() -> Result<UserProfile, UserError> {
    CONTEXT.with(|c| {
        let mut ctx = c.borrow_mut();
        let caller = ctx.env.caller();
        if caller == Principal::anonymous() {
            return Err(UserError::AnonymousNotAllowRegistering);
        }

        match ctx.user_service.get_user(&caller) {
            Some(u) => Ok(u),
            None => {
                let id = ctx.id;
                let now = ctx.env.now();
                let cmd = UserRegisterCommand {
                    email: "".to_string(),
                    name: "".to_string(),
                    memo: "".to_string(),
                };

                let user = cmd.build_profile(id, caller, UserStatus::Enable, now);

                match ctx.user_service.insert_user(user.clone()) {
                    Ok(_) => {
                        ctx.id += 1; // 注册成功，id + 1
                        Ok(user)
                    }
                    Err(e) => Err(e),
                }
            }
        }
    })
}

#[update(guard = "user_owner_guard")]
fn edit_user(cmd: UserEditCommand) -> Result<bool, UserError> {
    CONTEXT.with(|c| {
        let mut ctx = c.borrow_mut();
        let principal = ctx.env.caller();
        ctx.user_service.edit_user(cmd, &principal)
    })
}

#[update]
fn enable_user(principal: Principal) -> Result<bool, UserError> {
    CONTEXT.with(|c| {
        c.borrow_mut()
            .user_service
            .enable_user(&principal)
            .ok_or(UserError::UserNotFound)
    })
}

#[update]
fn disable_user(principal: Principal) -> Result<bool, UserError> {
    CONTEXT.with(|c| {
        c.borrow_mut()
            .user_service
            .disable_user(&principal)
            .ok_or(UserError::UserNotFound)
    })
}

#[update(guard = "user_owner_guard")]
fn update_wallet(wallet: Principal) -> Result<bool, UserError> {
    CONTEXT.with(|c| {
        let mut ctx = c.borrow_mut();
        let user = ctx.env.caller();

        ctx.user_service
            .update_wallet(&user, wallet)
            .ok_or(UserError::UserNotFound)
    })
}

#[update(guard = "user_owner_guard")]
fn delete_wallet() -> Result<bool, UserError> {
    CONTEXT.with(|c| {
        let mut ctx = c.borrow_mut();
        let user = ctx.env.caller();
        ctx.user_service
            .delete_wallet(&user)
            .ok_or(UserError::UserNotFound)
    })
}

#[query]
fn get_user(principal: Principal) -> Result<UserProfile, UserError> {
    CONTEXT
        .with(|c| c.borrow().user_service.get_user(&principal))
        .ok_or(UserError::UserNotFound)
}

#[query]
fn get_self() -> Result<UserProfile, UserError> {
    CONTEXT.with(|c| {
        let ctx = c.borrow();
        let caller = ctx.env.caller();
        ctx.user_service
            .get_user(&caller)
            .ok_or(UserError::UserNotFound)
    })
}

/// 获取用户经验值情况
#[query]
fn get_user_achievement(user: Principal) -> Result<Achievement, UserError> {
    CONTEXT.with(|c| {
        let ctx = c.borrow();
        query_achievement(ctx, user)
    })
}

/// 获取调用者经验值情况
#[query]
fn get_self_achievement() -> Result<Achievement, UserError> {
    CONTEXT.with(|c| {
        let ctx = c.borrow();
        let user = ctx.env.caller();
        query_achievement(ctx, user)
    })
}

/// 实时查询用户经验值
fn query_achievement(ctx: Ref<DaoContext>, user: Principal) -> Result<Achievement, UserError> {
    let owner = ctx
        .user_service
        .get_user(&user)
        .map(|u| u.owner)
        .ok_or(UserError::UserNotFound)?;

    let active = ctx.post_service.get_comment_count_by_user(user);
    let post_comment = ctx.post_service.get_post_comment_count_by_user(user);
    let reputation = ctx.reputation_service.get_reputation(&user).amount;
    let issued_bounty = ctx.post_service.get_issued_bounty_by_user(user);
    let received_bounty = ctx.post_service.get_received_bounty_by_user(user);

    let active_item =
        AchievementItem::new("active user".to_string(), "active user".to_string(), active);
    let post_comment_item = AchievementItem::new(
        "post comment".to_string(),
        "post comment".to_string(),
        post_comment,
    );
    let reputation_item = AchievementItem::new(
        "reputation".to_string(),
        "reputation".to_string(),
        reputation,
    );
    let issued_bounty_item = AchievementItem::new(
        "issued bounty".to_string(),
        "issued bounty".to_string(),
        issued_bounty,
    );
    let received_bounty_item = AchievementItem::new(
        "received bounty".to_string(),
        "received bounty".to_string(),
        received_bounty,
    );

    Ok(Achievement::new(
        owner,
        active_item,
        post_comment_item,
        reputation_item,
        issued_bounty_item,
        received_bounty_item,
    ))
}
