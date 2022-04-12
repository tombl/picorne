use keyberon::{
    action::Action::Custom,
    layout::{layout, Layers},
};
pub enum CustomAction {
    Reset,
    Bootsel,
}
use CustomAction::*;

// docs: https://github.com/TeXitoi/keyberon/pull/54
pub static LAYERS: Layers<CustomAction> = layout! {
    {
        [ A B C D E F ]
        [ G H I J K L ]
        [ M N O P Q R ]
        [ S T U n n n ]
    }
};
