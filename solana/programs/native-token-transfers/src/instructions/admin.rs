use anchor_lang::prelude::*;
use ntt_messages::chain_id::ChainId;

#[cfg(feature = "idl-build")]
use crate::messages::Hack;

use crate::{
    config::Config,
    error::NTTError,
    peer::NttManagerPeer,
    queue::{inbox::InboxRateLimit, outbox::OutboxRateLimit, rate_limit::RateLimitState},
    registered_transceiver::RegisteredTransceiver,
};

// * Transfer ownership

/// For safety reasons, transferring ownership is a 2-step process. The first step is to set the
/// new owner, and the second step is for the new owner to claim the ownership.
/// This is to prevent a situation where the ownership is transferred to an
/// address that is not able to claim the ownership (by mistake).
///
/// The transfer can be cancelled by the existing owner invoking the [`claim_ownership`]
/// instruction.
///
/// Alternatively, the ownership can be transferred in a single step by calling the
/// [`transfer_ownership_one_step_unchecked`] instruction. This can be dangerous because if the new owner
/// cannot actually sign transactions (due to setting the wrong address), the program will be
/// permanently locked. If the intention is to transfer ownership to a program using this instruction,
/// take extra care to ensure that the owner is a PDA, not the program address itself.
#[derive(Accounts)]
pub struct TransferOwnership<'info> {
    #[account(
        mut,
        has_one = owner,
    )]
    pub config: Account<'info, Config>,

    pub owner: Signer<'info>,

    /// CHECK: This account will be the signer in the [claim_ownership] instruction.
    new_owner: UncheckedAccount<'info>,

    #[account(
        seeds = [b"upgrade_lock"],
        bump,
    )]
    /// CHECK: The seeds constraint enforces that this is the correct address
    upgrade_lock: UncheckedAccount<'info>,
}

pub fn transfer_ownership(ctx: Context<TransferOwnership>) -> Result<()> {
    ctx.accounts.config.pending_owner = Some(ctx.accounts.new_owner.key());

    Ok(())
}

pub fn transfer_ownership_one_step_unchecked(ctx: Context<TransferOwnership>) -> Result<()> {
    ctx.accounts.config.pending_owner = None;
    ctx.accounts.config.owner = ctx.accounts.new_owner.key();

    Ok(())
}

// * Claim ownership

#[derive(Accounts)]
pub struct ClaimOwnership<'info> {
    #[account(
        mut,
        constraint = (
            config.pending_owner == Some(new_owner.key())
            || config.owner == new_owner.key()
        ) @ NTTError::InvalidPendingOwner
    )]
    pub config: Account<'info, Config>,

    #[account(
        seeds = [b"upgrade_lock"],
        bump,
    )]
    /// CHECK: The seeds constraint enforces that this is the correct address
    upgrade_lock: UncheckedAccount<'info>,

    pub new_owner: Signer<'info>,
}

pub fn claim_ownership(ctx: Context<ClaimOwnership>) -> Result<()> {
    ctx.accounts.config.pending_owner = None;
    ctx.accounts.config.owner = ctx.accounts.new_owner.key();

    Ok(())
}

// * Set peers
// TODO: update peers? should that be a separate instruction? take timestamp
// for modification? (for total ordering)

#[derive(Accounts)]
#[instruction(args: SetPeerArgs)]
pub struct SetPeer<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub owner: Signer<'info>,

    #[account(
        has_one = owner,
    )]
    pub config: Account<'info, Config>,

    #[account(
        init,
        space = 8 + NttManagerPeer::INIT_SPACE,
        payer = payer,
        seeds = [NttManagerPeer::SEED_PREFIX, args.chain_id.id.to_be_bytes().as_ref()],
        bump
    )]
    pub peer: Account<'info, NttManagerPeer>,

    #[account(
        init,
        space = 8 + InboxRateLimit::INIT_SPACE,
        payer = payer,
        seeds = [
            InboxRateLimit::SEED_PREFIX,
            args.chain_id.id.to_be_bytes().as_ref()
        ],
        bump,
    )]
    pub inbox_rate_limit: Account<'info, InboxRateLimit>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct SetPeerArgs {
    pub chain_id: ChainId,
    pub address: [u8; 32],
    pub limit: u64,
    /// The token decimals on the peer chain.
    pub token_decimals: u8,
}

pub fn set_peer(ctx: Context<SetPeer>, args: SetPeerArgs) -> Result<()> {
    ctx.accounts.peer.set_inner(NttManagerPeer {
        bump: ctx.bumps.peer,
        address: args.address,
        token_decimals: args.token_decimals,
    });

    ctx.accounts.inbox_rate_limit.set_inner(InboxRateLimit {
        bump: ctx.bumps.inbox_rate_limit,
        rate_limit: RateLimitState::new(args.limit),
    });
    Ok(())
}

// * Register transceivers

#[derive(Accounts)]
pub struct RegisterTransceiver<'info> {
    #[account(
        mut,
        has_one = owner,
    )]
    pub config: Account<'info, Config>,

    pub owner: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(executable)]
    /// CHECK: transceiver is meant to be a transceiver program. Arguably a `Program` constraint could be
    /// used here that wraps the Transceiver account type.
    pub transceiver: UncheckedAccount<'info>,

    #[account(
        init,
        space = 8 + RegisteredTransceiver::INIT_SPACE,
        payer = payer,
        seeds = [RegisteredTransceiver::SEED_PREFIX, transceiver.key().as_ref()],
        bump
    )]
    pub registered_transceiver: Account<'info, RegisteredTransceiver>,

    pub system_program: Program<'info, System>,
}

pub fn register_transceiver(ctx: Context<RegisterTransceiver>) -> Result<()> {
    let id = ctx.accounts.config.next_transceiver_id;
    ctx.accounts.config.next_transceiver_id += 1;
    ctx.accounts
        .registered_transceiver
        .set_inner(RegisteredTransceiver {
            bump: ctx.bumps.registered_transceiver,
            id,
            transceiver_address: ctx.accounts.transceiver.key(),
        });

    ctx.accounts.config.enabled_transceivers.set(id, true)?;
    Ok(())
}

// * Limit rate adjustment
#[derive(Accounts)]
pub struct SetOutboundLimit<'info> {
    #[account(
        constraint = config.owner == owner.key()
    )]
    pub config: Account<'info, Config>,

    pub owner: Signer<'info>,

    #[account(mut)]
    pub rate_limit: Account<'info, OutboxRateLimit>,
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct SetOutboundLimitArgs {
    pub limit: u64,
}

pub fn set_outbound_limit(
    ctx: Context<SetOutboundLimit>,
    args: SetOutboundLimitArgs,
) -> Result<()> {
    ctx.accounts.rate_limit.set_limit(args.limit);
    Ok(())
}

#[derive(Accounts)]
#[instruction(args: SetInboundLimitArgs)]
pub struct SetInboundLimit<'info> {
    #[account(
        constraint = config.owner == owner.key()
    )]
    pub config: Account<'info, Config>,

    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [
            InboxRateLimit::SEED_PREFIX,
            args.chain_id.id.to_be_bytes().as_ref()
        ],
        bump = rate_limit.bump
    )]
    pub rate_limit: Account<'info, InboxRateLimit>,
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct SetInboundLimitArgs {
    pub limit: u64,
    pub chain_id: ChainId,
}

pub fn set_inbound_limit(ctx: Context<SetInboundLimit>, args: SetInboundLimitArgs) -> Result<()> {
    ctx.accounts.rate_limit.set_limit(args.limit);
    Ok(())
}

// * Pausing
#[derive(Accounts)]
pub struct SetPaused<'info> {
    pub owner: Signer<'info>,

    #[account(
        mut,
        has_one = owner,
    )]
    pub config: Account<'info, Config>,
}

pub fn set_paused(ctx: Context<SetPaused>, paused: bool) -> Result<()> {
    ctx.accounts.config.paused = paused;
    Ok(())
}
