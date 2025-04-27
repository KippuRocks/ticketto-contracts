// Copyright 2025 The Kippu Authors
//
// Licensed under the Apache License, Version 2.0 (the "License")
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

#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub use ticketto_events::*;

#[cfg(test)]
mod tests;

#[ink::contract(env = KreivoApiEnvironment)]
mod ticketto_events {
    use codec::{Decode, Encode};
    use ink::codegen::Env;
    pub use kreivo_apis::{
        apis::{KreivoAPI, ListingsInventoriesAPI, ListingsItemsAPI},
        KreivoApi, KreivoApiEnvironment,
    };
    use ticketto_traits::WithMeteredBalance;
    pub use ticketto_types::{
        event_attributes, ticket_attributes, AttendancePolicy, Error, EventCapacity, EventId,
        EventInfo, EventName, ItemPrice, TicketId, TicketRestrictions,
    };
    use ticketto_types::{EventDates, EventState};
    /// Allows organisers to manage their events.
    ///
    /// This implementation of `Ticketto` heavily rely upon the Kreivo APIs. This means we'll use
    /// the Listings Inventory APIs (which under the hood are a wrapping for namespaced nonfungibles
    /// collections).
    ///
    /// Hence, this contract only stores the counter for the next event, since all the remaining
    /// properties will be stored as attributes to the inventory, for which the deposit balance
    /// would be provided by the account that instantiates the contract.
    #[ink(storage)]
    #[derive(Default, PartialEq, Eq)]
    pub struct TickettoEvents {
        /// A counter that indicates the next [`EventId`]
        pub(crate) next_event_id: EventId,
    }

    /// Emits when an event is registered.
    #[ink(event)]
    pub struct EventRegistered {
        id: EventId,
        organiser: AccountId,
    }

    /// Emits when a ticket is issued.
    #[ink(event)]
    pub struct TicketIssued {
        event: EventId,
        id: TicketId,
    }

    impl TickettoEvents {
        /// Default constructor.
        #[ink(constructor, selector = 0x00)]
        pub fn initialize() -> Self {
            Default::default()
        }

        /// Creates a new inventory for storing the event (and its details).
        ///
        /// The caller must assume the storage cost to create a new event, otherwise, a
        /// [`Deposit`][Error::Deposit] error will be thrown.
        #[ink(message, payable, selector = 0x01)]
        pub fn create_event(
            &mut self,
            name: EventName,
            capacity: EventCapacity,
            class: AttendancePolicy,
            maybe_price: Option<ItemPrice>,
            maybe_restrictions: Option<TicketRestrictions>,
        ) -> Result<(EventId, Balance), Error> {
            let event_id = self.get_event_id().ok_or(Error::Overflow)?;
            Self::with_metered_balance(&mut self.env(), &event_id, || {
                let caller = self.env().caller();
                <KreivoApi as KreivoAPI<_>>::Listings::create(&self.env(), &event_id)?;
                <KreivoApi as KreivoAPI<_>>::Listings::inventory_set_attribute(
                    &self.env(),
                    &event_id,
                    &event_attributes::ORGANISER,
                    &caller,
                )?;
                <KreivoApi as KreivoAPI<_>>::Listings::inventory_set_attribute(
                    &self.env(),
                    &event_id,
                    &event_attributes::NAME,
                    &name,
                )?;
                <KreivoApi as KreivoAPI<_>>::Listings::inventory_set_attribute(
                    &self.env(),
                    &event_id,
                    &event_attributes::CAPACITY,
                    &capacity,
                )?;
                <KreivoApi as KreivoAPI<_>>::Listings::inventory_set_attribute(
                    &self.env(),
                    &event_id,
                    &event_attributes::TICKET_CLASS,
                    &class,
                )?;
                if let Some(price) = maybe_price {
                    <KreivoApi as KreivoAPI<_>>::Listings::inventory_set_attribute(
                        &self.env(),
                        &event_id,
                        &event_attributes::TICKET_PRICE,
                        &price,
                    )?;
                }

                if let Some(ticket_restrictions) = maybe_restrictions {
                    <KreivoApi as KreivoAPI<_>>::Listings::inventory_set_attribute(
                        &self.env(),
                        &event_id,
                        &event_attributes::TICKET_RESTRICTIONS,
                        &ticket_restrictions,
                    )?;
                }

                self.env().emit_event(EventRegistered {
                    id: event_id,
                    organiser: self.env().caller(),
                });

                Ok(event_id)
            })
        }

        /// Edits the name of the event.
        ///
        /// The method fails if the event is ongoing or finished.
        #[ink(message, payable, selector = 0x02)]
        pub fn set_event_name(
            &mut self,
            event_id: EventId,
            name: EventName,
        ) -> Result<Balance, Error> {
            self.ensure_organiser(&event_id)?;
            self.with_state(&event_id, |state| {
                !matches!(state, EventState::Ongoing | EventState::Finished)
            })?;
            self.set_attribute(&event_id, &event_attributes::NAME, &name)
        }

        /// Edits the capacity of the event.
        ///
        /// The method fails if the event tickets are already on sale.
        #[ink(message, payable, selector = 0x03)]
        pub fn set_capacity(
            &mut self,
            event_id: EventId,
            capacity: EventCapacity,
        ) -> Result<Balance, Error> {
            self.ensure_organiser(&event_id)?;
            self.with_state(&event_id, |state| matches!(state, EventState::Created))?;
            self.set_attribute(&event_id, &event_attributes::TICKET_CLASS, &capacity)
        }

        /// Edits the class of the ticket.
        ///
        /// The method fails if the event tickets are already on sale.
        #[ink(message, payable, selector = 0x04)]
        pub fn set_ticket_class(
            &mut self,
            event_id: EventId,
            ticket_class: AttendancePolicy,
        ) -> Result<Balance, Error> {
            self.ensure_organiser(&event_id)?;
            self.with_state(&event_id, |state| matches!(state, EventState::Created))?;
            self.set_attribute(&event_id, &event_attributes::TICKET_CLASS, &ticket_class)
        }

        /// Edits the price of the ticket.
        ///
        /// The method fails if the event is ongoing or finished.
        #[ink(message, payable, selector = 0x05)]
        pub fn set_price(&mut self, event_id: EventId, price: ItemPrice) -> Result<Balance, Error> {
            self.ensure_organiser(&event_id)?;
            self.with_state(&event_id, |state| {
                matches!(state, EventState::Created | EventState::Sales)
            })?;
            self.set_attribute(&event_id, &event_attributes::TICKET_CLASS, &price)
        }

        /// Edits the restrictions of the ticket.
        ///
        /// The method fails if the event tickets are already on sale.
        #[ink(message, payable, selector = 0x06)]
        pub fn set_ticket_restrictions(
            &mut self,
            event_id: EventId,
            ticket_restrictions: TicketRestrictions,
        ) -> Result<Balance, Error> {
            self.ensure_organiser(&event_id)?;
            self.with_state(&event_id, |state| matches!(state, EventState::Created))?;
            self.set_attribute(
                &event_id,
                &event_attributes::TICKET_CLASS,
                &ticket_restrictions,
            )
        }

        /// Sets the dates of the event.
        ///
        /// The method fails if the event is ongoing or finished.
        #[ink(message, payable, selector = 0x07)]
        pub fn set_event_dates(
            &mut self,
            event_id: EventId,
            dates: EventDates,
        ) -> Result<Balance, Error> {
            Self::with_metered_balance(&mut self.env(), &event_id, || {
                self.ensure_organiser(&event_id)?;
                self.with_state(&event_id, |state| {
                    !matches!(state, EventState::Ongoing | EventState::Finished)
                })?;
                self.set_attribute(&event_id, &event_attributes::DATES, &dates)
            })
            .map(|(_, r)| r)
        }

        #[ink(message, payable, selector = 0x08)]
        pub fn bump_state(&mut self, event_id: EventId) -> Result<Balance, Error> {
            Self::with_metered_balance(&mut self.env(), &event_id, || {
                let state: EventState = <KreivoApi as KreivoAPI<_>>::Listings::inventory_attribute(
                    &self.env(),
                    &event_id,
                    &event_attributes::STATE,
                )
                .unwrap_or_default();

                let dates: EventDates = <KreivoApi as KreivoAPI<_>>::Listings::inventory_attribute(
                    &self.env(),
                    &event_id,
                    &event_attributes::STATE,
                )
                .ok_or(Error::DatesNotSet)?;

                let next_state = match state {
                    EventState::Created => {
                        // Only the organiser can bump to sales, once some dates have been set.
                        self.ensure_organiser(&event_id)?;
                        EventState::Sales
                    }
                    EventState::Sales => {
                        // Anyone can bump to ongoing, as long as the first date has been reached.
                        let (starts, _) = dates.first().ok_or(Error::DatesNotSet)?;
                        (starts <= &self.env().block_timestamp())
                            .then_some(())
                            .ok_or(Error::InvalidState)?;
                        EventState::Ongoing
                    }
                    // Anyone can bump to finished, as long as the last date has passed.
                    EventState::Ongoing => {
                        let (_, ends) = dates.last().ok_or(Error::DatesNotSet)?;
                        (ends <= &self.env().block_timestamp())
                            .then_some(())
                            .ok_or(Error::InvalidState)?;
                        EventState::Finished
                    }
                    // This is the last state, so it's final. Errors to prevent redundant
                    // overwriting and resources exhaustion.
                    EventState::Finished => Err(Error::InvalidState)?,
                };

                self.set_attribute(&event_id, &event_attributes::STATE, &next_state)?;
                Ok(())
            })
            .map(|(_, r)| r)
        }

        /// Permissionlessly issues a new ticket. The new ticket takes the parameters given by the
        /// `EventInfo`. Tickets can only be issued on the [`Sales`][EventState::Sales] and
        /// [`Ongoing`][EventState::Ongoing] period, until maximum capacity is reached.
        ///
        /// Once the event finishes, or the maximum capacity is reached, it won't be longer possible
        /// to issue more tickets.
        #[ink(message, payable, selector = 0x10)]
        pub fn issue_ticket(&mut self, event_id: EventId) -> Result<Balance, Error> {
            Self::with_metered_balance(&mut self.env(), &event_id, || {
				self.with_state(&event_id, |state| {
					!matches!(state, EventState::Sales | EventState::Ongoing)
				})?;
				let (organiser, name, capacity, attendance_policy, maybe_price, maybe_restrictions) =
					self.get_event_info(event_id).ok_or(Error::EventNotFound)?;
				let ticket_id = self.get_ticket_id(&event_id)?;

				(ticket_id < capacity)
					.then_some(())
					.ok_or(Error::MaxCapacity)?;

				<KreivoApi as KreivoAPI<_>>::Listings::publish(
					&self.env(),
					&event_id,
					&ticket_id,
					name,
					maybe_price,
				)?;
				<KreivoApi as KreivoAPI<_>>::Listings::item_set_attribute(
					&self.env(),
					&event_id,
					&ticket_id,
					&ticket_attributes::ATTENDANCE_POLICY,
					&attendance_policy,
				)?;

				if let Some(restrictions) = maybe_restrictions {
					if restrictions.cannot_resale {
						<KreivoApi as KreivoAPI<_>>::Listings::item_disable_resell(
							&self.env(),
							&event_id,
							&ticket_id,
						)?;
					}

					if restrictions.cannot_transfer {
						<KreivoApi as KreivoAPI<_>>::Listings::item_disable_transfer(
							&self.env(),
							&event_id,
							&ticket_id,
						)?;
					}
				}

				// We transfer the ownership of the ticket to the event organiser.
				// This makes possible for the organiser to get the earnings of a ticket once
				// sold via the `Orders` pallet.
				<KreivoApi as KreivoAPI<_>>::Listings::item_creator_transfer(
					&self.env(),
					&event_id,
					&ticket_id,
					&organiser,
				)?;

				self.env().emit_event(TicketIssued {
					event: event_id,
					id: ticket_id,
				});

				Ok(())
			})
				.map(|(_, r)| r)
        }

        /// A permissionless method, that allows anyone to cover the deposit account of the event.
        /// Fails if the given event does not exist.
        ///
        /// It is expected to have a zero difference, because it's a no-op (in storage terms).
        #[ink(message, payable, selector = 0xFFFFFFFF)]
        pub fn deposit(&mut self, event_id: EventId) -> Result<(), Error> {
            Self::with_metered_balance(&mut self.env(), &event_id, || {
                <KreivoApi as KreivoAPI<_>>::Listings::inventory_attribute::<_, AccountId>(
                    &self.env(),
                    &event_id,
                    &event_attributes::ORGANISER,
                )
                .map(|_| ())
                .ok_or(Error::EventNotFound)
            })
            .map(|_| ())
        }

        #[ink(message, selector = 0xFFFFFFFE)]
        pub fn get_event_info(&self, event_id: EventId) -> Option<EventInfo> {
            Some((
                self.get_attribute(&event_id, &event_attributes::ORGANISER)?,
                self.get_attribute(&event_id, &event_attributes::NAME)?,
                self.get_attribute(&event_id, &event_attributes::CAPACITY)?,
                self.get_attribute(&event_id, &event_attributes::TICKET_CLASS)?,
                self.get_attribute(&event_id, &event_attributes::TICKET_PRICE),
                self.get_attribute(&event_id, &event_attributes::TICKET_RESTRICTIONS),
            ))
        }
    }

    impl WithMeteredBalance for TickettoEvents {}

    impl TickettoEvents {
        fn get_event_id(&mut self) -> Option<EventId> {
            let event_id = self.next_event_id;
            self.next_event_id = self.next_event_id.checked_add(1)?;
            Some(event_id)
        }

        fn get_ticket_id(&self, event_id: &EventId) -> Result<TicketId, Error> {
            let next_ticket_id: TicketId =
                <KreivoApi as KreivoAPI<_>>::Listings::inventory_attribute(
                    &self.env(),
                    event_id,
                    &event_attributes::NEXT_TICKET_ID,
                )
                .unwrap_or_default();
            <KreivoApi as KreivoAPI<_>>::Listings::inventory_set_attribute(
                &self.env(),
                event_id,
                &event_attributes::NEXT_TICKET_ID,
                &next_ticket_id.checked_add(1).ok_or(Error::Overflow)?,
            )
            .unwrap_or_default();
            Ok(next_ticket_id)
        }

        fn ensure_organiser(&self, event_id: &EventId) -> Result<(), Error> {
            let event_organiser: ink::primitives::AccountId =
                <KreivoApi as KreivoAPI<_>>::Listings::inventory_attribute(
                    &self.env(),
                    event_id,
                    &event_attributes::ORGANISER,
                )
                .ok_or(Error::EventNotFound)?;
            // Ensure the caller is the event organiser.
            (event_organiser == self.env().caller())
                .then_some(())
                .ok_or(Error::NoPermission)
        }

        fn get_attribute<K: Encode, V: Encode + Decode>(
            &self,
            event_id: &EventId,
            attribute: &K,
        ) -> Option<V> {
            <KreivoApi as KreivoAPI<_>>::Listings::inventory_attribute(
                &self.env(),
                event_id,
                attribute,
            )
        }

        fn with_state(
            &self,
            event_id: &EventId,
            f: impl FnOnce(EventState) -> bool,
        ) -> Result<(), Error> {
            let state: EventState = <KreivoApi as KreivoAPI<_>>::Listings::inventory_attribute(
                &self.env(),
                event_id,
                &event_attributes::STATE,
            )
            .unwrap_or_default();

            f(state).then_some(()).ok_or(Error::InvalidState)
        }

        fn set_attribute<K: Encode, V: Encode>(
            &self,
            event_id: &EventId,
            attribute: &K,
            value: &V,
        ) -> Result<Balance, Error> {
            Self::with_metered_balance(&mut self.env(), event_id, || {
                <KreivoApi as KreivoAPI<_>>::Listings::inventory_set_attribute(
                    &self.env(),
                    event_id,
                    attribute,
                    value,
                )?;
                Ok(())
            })
            .map(|(_, r)| r)
        }
    }
}
