pub struct Position(pub i16, pub i16);

impl Position
{
    pub fn from_command(value: u32) -> Position
    {
        Position(value as i16, (value >> 16) as i16)
    }
}

pub struct Color(pub u8, pub u8, pub u8);

impl Color
{
    pub fn from_command(value: u32) -> Color
    {
        Color(value as u8, (value >> 8) as u8, (value >> 16) as u8)
    }
}

pub struct Renderer
{

}

impl Renderer
{
    pub fn new() -> Renderer
    {
        Renderer
        {

        }
    }

    pub fn push_triangle(&mut self, positions: &[Position], colors: &[Color])
    {

    }
}
