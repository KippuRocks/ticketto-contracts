use super::*;

use kreivo_apis::apis::KreivoApisError;

#[derive(Encode, Decode, TypeInfo)]
#[allow(clippy::cast_possible_truncation)]
#[repr(u8)]
pub enum Error {
    /// It was not possible to complete the operation due to low
    /// balance.
    LowBalance,
    /// The operation it's not possible due to overflow.
    Overflow,
    /// The given `EventId` is not found.
    EventNotFound,
    /// The given `TicketId` is not found.
    TicketNotFound,
    /// The caller does not have permissions to mutate the state of
    /// an event or a ticket.
    NoPermission,
    /// This action is not possible to be done until dates are set.
    DatesNotSet,
    /// This action is not possible to be done at the current state of the event.
    InvalidState,
    /// The maximum capacity of the event was reached. It is not
    /// possible to issue a new ticket.
    MaxCapacity,
    /// The ticket is trying to be used outside the times of the event.
    AttendanceOutOfDates,
    /// The maximum number of attendances has been excedeed for this ticket.
    MaxAttendances,
    /// The ticket is restricted for transferring.
    CannotTransfer,
    /// There is not a pending transfer in progress for this ticket.
    NoPendingTransfer,
    /// An error occurred on the Kreivo APIs side.
    KreivoApiError(KreivoApisError),
    /// A generic error that cannot be categorized.
    Other([u8; 32]),
}

impl From<KreivoApisError> for Error {
    fn from(value: KreivoApisError) -> Self {
        Self::KreivoApiError(value)
    }
}
