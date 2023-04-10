use anyhow::{Error, Result};
use bitflags::bitflags;
use std::convert::TryFrom;
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Perms: u16 {
        const UR = 0o400;
        const UW = 0o200;
        const UX = 0o100;
        const GR = 0o040;
        const GW = 0o020;
        const GX = 0o010;
        const OR = 0o004;
        const OW = 0o002;
        const OX = 0o001;
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Perm: u8 {
        const R = 4;
        const W = 2;
        const X = 1;

        const RWX = 7;
        const RW = 6;
        const RX = 5;
        const WX = 3;
    }
}

impl Perms {
    pub fn map(
        self: &Perms,
        user: fn(Perm) -> Perm,
        group: fn(Perm) -> Perm,
        other: fn(Perm) -> Perm,
    ) -> Perms {
        Perms::from(user(self.user()), group(self.group()), other(self.other()))
    }

    pub fn map_user(self: &Perms, f: fn(Perm) -> Perm) -> Perms {
        self.map(f, |x| x, |x| x)
    }

    pub fn map_group(self: &Perms, f: fn(Perm) -> Perm) -> Perms {
        self.map(|x| x, f, |x| x)
    }

    pub fn map_other(self: &Perms, f: fn(Perm) -> Perm) -> Perms {
        self.map(|x| x, |x| x, f)
    }

    pub fn user(self: &Perms) -> Perm {
        Perm::from_bits_truncate((self.bits() >> 6) as u8)
    }

    pub fn group(self: &Perms) -> Perm {
        Perm::from_bits_truncate((self.bits() >> 3) as u8)
    }

    pub fn other(self: &Perms) -> Perm {
        Perm::from_bits_truncate(self.bits() as u8)
    }

    pub fn from(user: Perm, group: Perm, other: Perm) -> Perms {
        Perms::from_bits_truncate(
            ((user.bits() as u16) << 6) | ((group.bits() as u16) << 3) | other.bits() as u16,
        )
    }
}

pub const MODE_MASK: u32 = 0o7777;

impl TryFrom<Permissions> for Perms {
    type Error = Error;

    fn try_from(permissions: Permissions) -> Result<Self> {
        let mode = (permissions.mode() & MODE_MASK) as u16;
        match Perms::from_bits(mode) {
            Some(perms) => Ok(perms),
            None => Err(anyhow!("Unknown bits set, possibly sticky: {}", mode)),
        }
    }
}

impl From<Perms> for Permissions {
    fn from(perms: Perms) -> Self {
        Permissions::from_mode(perms.bits() as u32)
    }
}

#[cfg(test)]
mod test {

    use crate::perm::{Perm, Perms};
    use std::fs::Permissions;
    use std::os::unix::fs::PermissionsExt;

    fn all_perms<F: Fn(Perm) -> ()>(f: F) {
        for p in 0..7 {
            f(Perm::from_bits_truncate(p))
        }
    }

    fn all_permss<F: Fn(Perms) -> ()>(f: F) {
        for p in 0..511 {
            f(Perms::from_bits_truncate(p))
        }
    }

    #[test]
    fn test_convert1() {
        all_permss(|perms| {
            assert_eq!(
                perms,
                Perms::from(perms.user(), perms.group(), perms.other())
            )
        })
    }

    #[test]
    fn test_convert2() {
        all_perms(|user| {
            all_perms(|group| {
                all_perms(|other| {
                    let perms = Perms::from(user, group, other);
                    assert_eq!(user, perms.user());
                    assert_eq!(group, perms.group());
                    assert_eq!(other, perms.other())
                })
            })
        })
    }

    #[test]
    fn test_convert3() {
        all_permss(|perms| {
            let lib: Permissions = perms.clone().into();
            let actual: Perms = lib.try_into().unwrap();
            assert_eq!(perms, actual)
        })
    }

    #[test]
    fn test_combined() {
        assert_eq!(Perm::R | Perm::W | Perm::X, Perm::RWX);
        assert_eq!(Perm::R | Perm::W, Perm::RW);
        assert_eq!(Perm::R | Perm::X, Perm::RX);
        assert_eq!(Perm::W | Perm::X, Perm::WX)
    }

    #[test]
    fn test_map_id() {
        all_permss(|perms| assert_eq!(perms, perms.map(|x| x, |x| x, |x| x)))
    }

    #[test]
    fn test_map_user_id() {
        all_permss(|perms| assert_eq!(perms, perms.map_user(|x| x)))
    }

    #[test]
    fn test_map_group_id() {
        all_permss(|perms| assert_eq!(perms, perms.map_group(|x| x)))
    }

    #[test]
    fn test_map_other_id() {
        all_permss(|perms| assert_eq!(perms, perms.map_other(|x| x)))
    }

    #[test]
    fn test_map() {
        all_permss(|perms| {
            let expected = Perms::from(
                perms.user() | Perm::X,
                perms.group() | Perm::W,
                perms.other() | Perm::R,
            );
            let actual = perms.map(|x| x | Perm::X, |x| x | Perm::W, |x| x | Perm::R);
            assert_eq!(expected, actual)
        })
    }

    #[test]
    #[should_panic(expected = "sticky")]
    fn test_unix_sticky() {
        Perms::try_from(Permissions::from_mode(0o1111)).unwrap();
    }

    #[test]
    fn test_unix_clear() {
        let result: Perms = Permissions::from_mode(0o10111).try_into().unwrap();
        assert_eq!(Perms::from_bits_truncate(0o111), result)
    }
}
