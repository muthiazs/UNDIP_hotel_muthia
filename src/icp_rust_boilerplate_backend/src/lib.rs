#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Room {
    id: u64,
    floor: u32,
    room_number: u32,
    is_available: bool,
    check_in_date: u64,
    check_out_date: u64,
}

impl Storable for Room {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Room {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static STORAGE: RefCell<StableBTreeMap<u64, Room, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct RoomPayload {
    floor: u32,
    room_number: u32,
    check_in_date: u64,
    check_out_date: u64,
}

#[ic_cdk::query]
fn get_room(id: u64) -> Result<Room, Error> {
    match _get_room(&id) {
        Some(room) => Ok(room),
        None => Err(Error::NotFound {
            msg: format!("a room with id={} not found", id),
        }),
    }
}

#[ic_cdk::update]
fn add_room(room: RoomPayload) -> Option<Room> {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");
    let room = Room {
        id,
        floor: room.floor,
        room_number: room.room_number,
        is_available: true,
        check_in_date: room.check_in_date,
        check_out_date: room.check_out_date,
    };
    do_insert(&room);
    Some(room)
}

#[ic_cdk::update]
fn update_room(id: u64, payload: RoomPayload) -> Result<Room, Error> {
    match STORAGE.with(|service