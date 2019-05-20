use bidir_map::BidirMap;
use piston_window::{
    Button,
    Key,
};

////////////////////////////////////////////////////////////////////////////////

#[derive(Hash, Debug, Copy, Clone, Eq, PartialEq)]
pub enum BindRoles {
    BT_A,
    BT_B,
    BT_C,
    BT_D,
    FX_L,
    FX_R,
    KN_L_CW,
    KN_L_CCW,
    KN_R_CW,
    KN_R_CCW,
    START,

    BACK,
}

impl BindRoles {
    pub fn default_keyboard_binding() -> BidirMap<BindRoles, ComposedKeystroke>
    {
        use self::GeneralizedKeystroke as GK;
        use piston_window::keyboard::Key;
        use BindRoles::*;

        let mut map = BidirMap::new();

        macro_rules! CK_GENERATOR {
            [ $( ($bind: ident , $id: ident) ),+ ] => {
                vec![
                    $(
                        (
                            $bind,
                            ComposedKeystroke::new(GK::Keyboard(Key::$id))
                        )
                    ),+
                ]
            }
        }

        CK_GENERATOR![
            (BT_A, D),
            (BT_B, F),
            (BT_C, J),
            (BT_D, K),
            (FX_L, V),
            (FX_R, N),
            (KN_L_CW, R),
            (KN_L_CCW, E),
            (KN_R_CW, I),
            (KN_R_CCW, U),
            (START, Return),
            (BACK, Escape)
        ]
        .into_iter()
        .for_each(|(role, ks)| {
            map.insert(role, ks);
        });

        map
    }
}

#[derive(Hash, Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum GeneralizedKeystroke {
    Keyboard(Key),
    Controller(u8),
}

impl GeneralizedKeystroke {
    pub fn is(
        &self,
        button: &Button,
    ) -> bool
    {
        use Button as B;
        use GeneralizedKeystroke as GK;

        match (self, button) {
            (GK::Keyboard(k1), B::Keyboard(k2)) => k1 == k2,
            (GK::Controller(k), B::Controller(c)) => *k == c.button,
            _ => false,
        }
    }

    pub fn from_button(button: &Button) -> Option<GeneralizedKeystroke> {
        use Button as B;
        use GeneralizedKeystroke as GK;

        match button {
            B::Keyboard(k) => Some(GK::Keyboard(k.clone())),
            B::Controller(c) => Some(GK::Controller(c.button.clone())),
            _ => None,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct ComposedKeystroke(Vec<GeneralizedKeystroke>);

impl ComposedKeystroke {
    pub fn new(key: GeneralizedKeystroke) -> ComposedKeystroke {
        ComposedKeystroke(vec![key])
    }

    pub fn add(
        &mut self,
        key: GeneralizedKeystroke,
    )
    {
        let bin_srch_idx = self.0.binary_search(&key);

        // check if the key already exists
        match bin_srch_idx {
            Ok(_) => return,
            Err(e) => self.0.insert(e, key),
        }
    }

    pub fn contains(
        &self,
        key: GeneralizedKeystroke,
    ) -> bool
    {
        self.0.binary_search(&key).is_ok()
    }
}
