use super::*;

#[allow(clippy::cast_possible_truncation)]
#[derive(Encode, Decode)]
pub enum TickettoAttribute {
	Event(EventAttribute),
	Ticket(TicketAttribute),
}

#[allow(clippy::cast_possible_truncation)]
#[derive(Decode, Encode, Debug, Clone, PartialEq, Eq)]
pub enum EventAttribute {
	DepositBalance,
	Name,
	Organiser,
	State,
	Dates,
	Capacity,
	TicketClass(Vec<u8>),
	NextTicketId,
}

impl From<EventAttribute> for TickettoAttribute {
	fn from(value: EventAttribute) -> TickettoAttribute {
		TickettoAttribute::Event(value)
	}
}

#[derive(Encode, Decode, Debug)]
pub enum TicketAttribute {
	Attendances,
	AttendancePolicy,
	PendingTransfer,
}

impl From<TicketAttribute> for TickettoAttribute {
	fn from(value: TicketAttribute) -> Self {
		TickettoAttribute::Ticket(value)
	}
}

