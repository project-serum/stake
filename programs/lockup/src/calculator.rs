//! Utility functions for calculating unlock schedules for a vesting account.

use crate::Vesting;

pub fn available_for_withdrawal(vesting: &Vesting, current_ts: i64) -> u64 {
    std::cmp::min(outstanding_vested(vesting, current_ts), balance(vesting))
}

// The amount of funds currently in the vault.
fn balance(vesting: &Vesting) -> u64 {
    vesting
        .outstanding
        .checked_sub(vesting.whitelist_owned)
        .unwrap()
}

// The amount of outstanding locked tokens vested. Note that these
// tokens might have been transferred to whitelisted programs.
fn outstanding_vested(vesting: &Vesting, current_ts: i64) -> u64 {
    total_vested(vesting, current_ts)
        .checked_sub(withdrawn_amount(vesting))
        .unwrap()
}

// Returns the amount withdrawn from this vesting account.
fn withdrawn_amount(vesting: &Vesting) -> u64 {
    vesting
        .start_balance
        .checked_sub(vesting.outstanding)
        .unwrap()
}

// Returns the total vested amount up to the given ts, assuming zero
// withdrawals and zero funds sent to other programs.
fn total_vested(vesting: &Vesting, current_ts: i64) -> u64 {
    if current_ts < vesting.start_ts {
        0
    } else if current_ts >= vesting.end_ts {
        vesting.start_balance
    } else {
        linear_unlock(vesting, current_ts)
    }
}

// Assumes `current_ts` < `vesting.end_ts`.
fn linear_unlock(vesting: &Vesting, current_ts: i64) -> u64 {
    // Signed division not supported.
    let current_ts = current_ts as f64;
    let start_ts = vesting.start_ts as f64;
    let end_ts = vesting.end_ts as f64;

    // The length of a single vesting period.
    // Invariant: period_count <= (end_ts - start_ts).
    let period_secs: f64 = (end_ts - start_ts) / (vesting.period_count as f64);

    // The period the current_ts is in (floor divides).
    // Invariant: current_ts >= start_ts.
    let current_period: u64 = ((current_ts - start_ts) / period_secs) as u64;

    // Reward per period.
    let reward_per_period: f64 = (vesting.start_balance as f64) / (vesting.period_count as f64);

    // Rounds the total reward down to the nearest integer, since we can't
    // pay out fractional rewards.
    ((current_period as f64) * reward_per_period) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::solana_program::pubkey::Pubkey;

    // Window = 10 seconds.
    // Period count = 2.
    // =>
    // Every 5 seconds 2.5 is vested.
    #[test]
    fn vesting_window_evenly_divisible_by_period_count() {
        let v = create_vesting(5, 10, 20, 2);
        let cases = vec![
            [0, 0], // Before vesting begins.
            [9, 0],
            [10, 0], // Vesting begins.
            [11, 0],
            [12, 0],
            [13, 0],
            [14, 0],
            [15, 2], // 2.5 is vested (floor).
            [16, 2],
            [17, 2],
            [18, 2],
            [19, 2],
            [20, 5], // All vested.
            [21, 5],
        ];
        run_test(v, cases);
    }

    // Window = 11 seconds.
    // Period count = 2.
    // =>
    // Every 5.5 seconds 2.5 is vested.
    #[test]
    fn vesting_window_not_evenly_divisble_by_period_count() {
        let v = create_vesting(5, 10, 21, 2);
        let cases = vec![
            [10, 0], // Vesting begins.
            [11, 0],
            [12, 0],
            [13, 0],
            [14, 0],
            [15, 0],
            [16, 2], // 2.5 vested.
            [17, 2],
            [18, 2],
            [19, 2],
            [20, 2],
            [21, 5], // All vested.
            [22, 5],
        ];
        run_test(v, cases);
    }

    // Winow = 11 seconds.
    // Period_count = 6.
    // =>
    // Every 1.83 seconds about 16.67 is vested.
    #[test]
    fn cumulative_remainder() {
        let v = create_vesting(100, 30, 41, 6);
        let cases = vec![
            [30, 0], // Vesting begins.
            [31, 0],
            [32, 16], // 16.67 @ 1.83 seconds.
            [33, 16],
            [34, 33], // 33.34 @ 3.66 seconds.
            [35, 33],
            [36, 50], // 50.01 @ 5.49 seconds.
            [37, 50],
            [38, 66], // 66.68 @ 7.32 seconds.
            [39, 66],
            [40, 83],  // 83.35 @ 9.15 seconds.
            [41, 100], // 100 @ 11 seconds.
        ];
        run_test(v, cases);
    }

    // Each case is an array consisting of
    // [start_balance, start_ts, end_ts, period_count, current_ts, total_vested].
    fn run_test(v: Vesting, cases: Vec<[u64; 2]>) {
        for c in cases.iter() {
            println!("Case: {:?}", c);
            let r = total_vested(&v, c[0] as i64);
            assert_eq!(r, c[1])
        }
    }

    fn create_vesting(
        start_balance: u64,
        start_ts: i64,
        end_ts: i64,
        period_count: u64,
    ) -> Vesting {
        Vesting {
            beneficiary: Pubkey::new_unique(),
            mint: Pubkey::new_unique(),
            vault: Pubkey::new_unique(),
            grantor: Pubkey::new_unique(),
            outstanding: 0,
            start_balance,
            created_ts: 0,
            start_ts,
            end_ts,
            period_count,
            whitelist_owned: 0,
            nonce: 0,
            realizor: None,
        }
    }
}
