/// 画像の回転角度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    None,// 0度 
    Right,// 90度
    Rotate180,// 180度
    Left,// 270度
}

impl Rotation{
    /// 右に90度回転する
    pub fn rotate_right(self) -> Self {
        match self {
            Rotation::None => Rotation::Right,
            Rotation::Right => Rotation::Rotate180,
            Rotation::Rotate180 => Rotation::Left,
            Rotation::Left => Rotation::None,   
        }
    }
    
    /// 左に90度回転する
    pub fn rotate_left(self) -> Self {
        match self {
            Rotation::None => Rotation::Left,
            Rotation::Right => Rotation::None,
            Rotation::Rotate180 => Rotation::Right,
            Rotation::Left => Rotation::Rotate180,
        }
    }

    ///　角度を取得
    pub fn degrees(self) -> i32 {
        match self {
            Rotation::None => 0,
            Rotation::Right => 90,
            Rotation::Rotate180 => 180,
            Rotation::Left => 270,
        }
    }

    /// 縦横を入れ替える必要があるかどうかを判定
    pub fn is_sadeways_swap(self) -> bool {
        match self {
            Rotation::None => false,
            Rotation::Right => true,
            Rotation::Rotate180 => false,
            Rotation::Left => true,
        }
    }

}
