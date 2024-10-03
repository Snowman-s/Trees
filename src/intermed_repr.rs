use crate::structs::{Block, QuoteStyle};

/*
中間表現バイトコードは、以下の繰り返し
子供は、深さ優先で並べて記述 (先頭の子ほど早く記述される)
- ブロック種類
- プロシージャ名長さ
- プロシージャ名
- 子供ブロックの数
- 子が@かどうか[子供の数]
*/
#[repr(u8)]
enum ArgPlugType {
  // ふつう
  Normal = 0x0,
  // @
  Expand = 0x1,
}

#[repr(u8)]
enum BlockType {
  // ふつう
  Normal = 0x1,
  // クォート
  Quote = 0x2,
  // クロージャ
  Closure = 0x3,
}

impl Block {
  pub fn to_intermed_repr(&self) -> Vec<u8> {
    // 生成された中間表現
    let mut ret: Vec<u8> = Vec::new();

    let mut stack = vec![self];

    while let Some(seeing) = stack.pop() {
      ret.push(match seeing.quote {
        QuoteStyle::None => BlockType::Normal,
        QuoteStyle::Quote => BlockType::Quote,
        QuoteStyle::Closure => BlockType::Closure,
      } as u8);
      ret.extend(u32::try_from(seeing.proc_name.len()).unwrap().to_be_bytes());
      ret.extend(seeing.proc_name.as_bytes());

      // 次の子について、expand を判定する。
      if !seeing.args.is_empty() {
        for (expand, _) in seeing.args.iter() {
          ret.push(match expand {
            true => ArgPlugType::Expand,
            false => ArgPlugType::Normal,
          } as u8);
        }
        // 引数を逆順に追加して、スタックから順番に取り出す
        for (_, child_block) in seeing.args.iter().rev() {
          stack.push(child_block);
        }
      }

      ret.push(seeing.args.len() as u8);
    }

    ret
  }
}

#[cfg(test)]
mod tests {
  use crate::structs::{Block, QuoteStyle};

  #[test]
  fn one_block_to_intermediate() {
    let block = Block {
      quote: QuoteStyle::None,
      proc_name: "aaaa".into(),
      args: vec![],
    };

    let im = block.to_intermed_repr();

    assert_eq!(im, vec![1, 0, 0, 0, 4, 97, 97, 97, 97, 0]);
  }

  #[test]
  fn nest_block_to_intermediate() {
    let block = Block {
      quote: QuoteStyle::None,
      proc_name: "a".into(),
      args: vec![
        (
          true,
          Box::new(Block {
            quote: QuoteStyle::None,
            proc_name: "b".into(),
            args: vec![(
              false,
              Box::new(Block {
                quote: QuoteStyle::Quote,
                proc_name: "c".into(),
                args: vec![],
              }),
            )],
          }),
        ),
        (
          false,
          Box::new(Block {
            quote: QuoteStyle::Closure,
            proc_name: "d".into(),
            args: vec![],
          }),
        ),
      ],
    };

    let im = block.to_intermed_repr();

    assert_eq!(
      im,
      vec![
        1, 0, 0, 0, 1, 97, 1, 0, 2, // type="normal",  name_len=1, name="a", @, not @, child=2,
        1, 0, 0, 0, 1, 98, 0, 1, // type="normal", name_len=1, name="b", not @, child=1,
        2, 0, 0, 0, 1, 99, 0, //  type="quote", name_len=1, name="c",  child=0,
        3, 0, 0, 0, 1, 100, 0, // type="closure",  name_len=1, name="d", child=0,
      ]
    );
  }
}
