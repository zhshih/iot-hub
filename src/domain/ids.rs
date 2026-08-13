use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

macro_rules! define_id_newtype {
    ($name:ident) => {
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            Serialize,
            Deserialize,
            sqlx::Type,
            utoipa::ToSchema,
        )]
        #[serde(transparent)]
        #[sqlx(transparent)]
        #[schema(value_type = String, format = "uuid")]
        pub struct $name(pub Uuid);

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            pub fn nil() -> Self {
                Self(Uuid::nil())
            }

            pub fn into_inner(self) -> Uuid {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Uuid::from_str(s).map(Self)
            }
        }

        impl From<Uuid> for $name {
            fn from(id: Uuid) -> Self {
                Self(id)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }
    };
}

define_id_newtype!(UserId);
define_id_newtype!(DeviceId);

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::DeserializeOwned;

    trait IdNewtype {
        fn new() -> Self;
        fn nil() -> Self;
    }

    macro_rules! impl_id_newtype_for_test {
        ($name:ident) => {
            impl IdNewtype for $name {
                fn new() -> Self {
                    $name::new()
                }
                fn nil() -> Self {
                    $name::nil()
                }
            }
        };
    }

    impl_id_newtype_for_test!(UserId);
    impl_id_newtype_for_test!(DeviceId);

    fn assert_id_roundtrips<T>()
    where
        T: IdNewtype
            + Copy
            + PartialEq
            + fmt::Debug
            + fmt::Display
            + FromStr
            + Serialize
            + DeserializeOwned
            + From<Uuid>
            + Into<Uuid>,
        T::Err: fmt::Debug,
    {
        assert_ne!(T::new(), T::new(), "new() should produce distinct ids");

        let nil: Uuid = T::nil().into();
        assert_eq!(nil, Uuid::nil());

        let id = T::new();
        let roundtripped: T = id.to_string().parse().unwrap();
        assert_eq!(roundtripped, id);

        assert!("not-a-uuid".parse::<T>().is_err());

        let id = T::new();
        let id_as_uuid: Uuid = id.into();
        assert_eq!(
            serde_json::to_string(&id).unwrap(),
            serde_json::to_string(&id_as_uuid).unwrap(),
            "JSON wire format must be identical to a plain Uuid"
        );
        let deserialized: T = serde_json::from_str(&serde_json::to_string(&id).unwrap()).unwrap();
        assert_eq!(deserialized, id);

        let raw = Uuid::new_v4();
        let via_newtype: Uuid = T::from(raw).into();
        assert_eq!(via_newtype, raw);
    }

    #[test]
    fn user_id_roundtrips() {
        assert_id_roundtrips::<UserId>();
    }

    #[test]
    fn device_id_roundtrips() {
        assert_id_roundtrips::<DeviceId>();
    }
}
