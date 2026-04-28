use std::ops::{BitAnd, BitOr, BitOrAssign};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Buttons(u16);

impl Buttons {
    pub(super) const EMPTY: Self = Self(0);
    pub(super) const DPAD_UP: Self = Self(0x0001);
    pub(super) const DPAD_DOWN: Self = Self(0x0002);
    pub(super) const DPAD_LEFT: Self = Self(0x0004);
    pub(super) const DPAD_RIGHT: Self = Self(0x0008);
    pub(super) const START: Self = Self(0x0010);
    pub(super) const BACK: Self = Self(0x0020);
    pub(super) const LEFT_THUMB: Self = Self(0x0040);
    pub(super) const RIGHT_THUMB: Self = Self(0x0080);
    pub(super) const LEFT_SHOULDER: Self = Self(0x0100);
    pub(super) const RIGHT_SHOULDER: Self = Self(0x0200);
    pub(super) const HOME: Self = Self(0x0400);
    pub(super) const A: Self = Self(0x1000);
    pub(super) const B: Self = Self(0x2000);
    pub(super) const X: Self = Self(0x4000);
    pub(super) const Y: Self = Self(0x8000);

    pub(super) fn is_empty(self) -> bool {
        self == Self::EMPTY
    }

    pub(super) fn intersects(self, other: Self) -> bool {
        !(self & other).is_empty()
    }
}

impl BitOr for Buttons {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Buttons {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for Buttons {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}
