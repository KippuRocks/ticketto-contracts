// Copyright 2025 The Kippu Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]

use ink::EnvAccess;
use ink_env::Environment;
use kreivo_apis::apis::{KreivoAPI, ListingsInventoriesAPI};
use kreivo_apis::{KreivoApi, KreivoApiEnvironment};
use ticketto_types::{event_attributes, Error, EventId};

type Balance = <KreivoApiEnvironment as Environment>::Balance;

/// Since we need to keep the balance each event organiser deposits for covering the costs of
/// reserving balances, we define this trait to handle the metering of the balances that are taken
/// during the executing of the contract, and reverts the execution of it in case the balance is not
/// enough to cover for the costs.
///
/// This is to ensure that no event organiser would be subsidising another event's costs.
pub trait WithMeteredBalance {
    /// Measures the changes in balance, and enforces the execution doesn't incur in
    /// [`LowBalance`][Error::LowBalance].
    fn with_metered_balance<R>(
        env: &mut EnvAccess<KreivoApiEnvironment>,
        event_id: &EventId,
        f: impl FnOnce() -> Result<R, Error>,
    ) -> Result<(R, Balance), Error> {
        let pre_balance = env.clone().balance();
        let transferred_value = env.clone().transferred_value();

        let event_balance: Balance = <KreivoApi as KreivoAPI<_>>::Listings::inventory_attribute(
            env,
            event_id,
            &event_attributes::DEPOSIT_BALANCE,
        )
        .unwrap_or_default();

        let r = f()?;

        let post_balance = env.clone().balance();

        // No balance was spent, or even some balance was released. Also, some balance may have
        // been provided.
        let difference = if post_balance >= pre_balance {
            let difference = post_balance
                .checked_sub(pre_balance)
                .ok_or(Error::LowBalance)?;

            <KreivoApi as KreivoAPI<_>>::Listings::inventory_set_attribute(
                env,
                event_id,
                &event_attributes::DEPOSIT_BALANCE,
                &event_balance
                    .checked_add(transferred_value)
                    .ok_or(Error::Overflow)?
                    .checked_add(difference)
                    .ok_or(Error::Overflow)?,
            )?;

            difference
        } else {
            let difference = pre_balance
                .checked_sub(post_balance)
                .ok_or(Error::LowBalance)?;

            <KreivoApi as KreivoAPI<_>>::Listings::inventory_set_attribute(
                env,
                event_id,
                &event_attributes::DEPOSIT_BALANCE,
                &event_balance
                    .checked_add(transferred_value)
                    .ok_or(Error::Overflow)?
                    .checked_sub(difference)
                    .ok_or(Error::LowBalance)?,
            )?;

            difference
        };

        Ok((r, difference))
    }
}
