// keyberon's matrix, but with rows and columns swapped
// i could just use theirs and invert PressedKeys though

use embedded_hal::digital::v2::{InputPin, OutputPin};
use keyberon::matrix::PressedKeys;

pub struct Matrix<C, R, const CS: usize, const RS: usize>
where
    C: OutputPin,
    R: InputPin,
{
    cols: [C; CS],
    rows: [R; RS],
}

impl<C, R, const CS: usize, const RS: usize> Matrix<C, R, CS, RS>
where
    C: OutputPin,
    R: InputPin,
{
    pub fn new<E>(cols: [C; CS], rows: [R; RS]) -> Result<Self, E>
    where
        C: OutputPin<Error = E>,
        R: InputPin<Error = E>,
    {
        let mut res = Self { cols, rows };
        res.clear()?;
        Ok(res)
    }
    pub fn clear<E>(&mut self) -> Result<(), E>
    where
        C: OutputPin<Error = E>,
        R: InputPin<Error = E>,
    {
        for c in self.cols.iter_mut() {
            c.set_high()?;
        }
        Ok(())
    }
    pub fn get<E>(&mut self) -> Result<PressedKeys<CS, RS>, E>
    where
        C: OutputPin<Error = E>,
        R: InputPin<Error = E>,
    {
        let mut keys = PressedKeys::default();

        for (ci, col) in self.cols.iter_mut().enumerate() {
            col.set_low()?;

            // cortex_m::asm::delay(100);

            for (ri, row) in self.rows.iter().enumerate() {
                if row.is_low()? {
                    keys.0[ri][ci] = true;
                }
            }
            col.set_high()?;
        }
        Ok(keys)
    }
}
