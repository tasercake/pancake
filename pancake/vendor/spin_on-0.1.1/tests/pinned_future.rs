//! Confirm that stack pinning preserves an immovable future across repeated polls.
#![forbid(unsafe_code)]

use core::cell::Cell;
use core::future::Future;
use core::marker::PhantomPinned;
use core::pin::Pin;
use core::task::{Context, Poll};

struct AddressChecked<'a> {
    first_address: Cell<*const Self>,
    polls: Cell<u8>,
    dropped: &'a Cell<u8>,
    _pin: PhantomPinned,
}

impl Future for AddressChecked<'_> {
    type Output = Result<u8, &'static str>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_ref().get_ref();
        let current = core::ptr::from_ref(this);
        if this.first_address.get().is_null() {
            this.first_address.set(current);
        } else if this.first_address.get() != current {
            return Poll::Ready(Err("future moved after its first poll"));
        }
        let count = this.polls.get().saturating_add(1);
        this.polls.set(count);
        if count == 4 {
            Poll::Ready(Ok(count))
        } else {
            let cloned = cx.waker().clone();
            cloned.wake_by_ref();
            cloned.wake();
            Poll::Pending
        }
    }
}

impl Drop for AddressChecked<'_> {
    fn drop(&mut self) {
        self.dropped.set(self.dropped.get().saturating_add(1));
    }
}

#[test]
fn immovable_future_keeps_address_and_drops_once() -> Result<(), &'static str> {
    let dropped = Cell::new(0);
    let output = spin_on::spin_on(AddressChecked {
        first_address: Cell::new(core::ptr::null()),
        polls: Cell::new(0),
        dropped: &dropped,
        _pin: PhantomPinned,
    });
    if output != Ok(4) {
        return Err("executor did not preserve the future through four polls");
    }
    if dropped.get() != 1 {
        return Err("future was not dropped exactly once");
    }
    Ok(())
}
