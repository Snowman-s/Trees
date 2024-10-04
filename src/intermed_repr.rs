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

      ret.push(seeing.args.len() as u8);

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
    }

    ret
  }

  // 一旦これで。そのうちブロックに戻さずに実行できるようにする。
  pub fn from_intermed_repr(code: &[u8]) -> Block {
    let mut index = 0;
    let mut stack: Vec<(Block, usize)> = Vec::new();

    // 最初のルートブロックを読み込む
    let quote = match code[index] {
      x if x == BlockType::Normal as u8 => QuoteStyle::None,
      x if x == BlockType::Quote as u8 => QuoteStyle::Quote,
      x if x == BlockType::Closure as u8 => QuoteStyle::Closure,
      x => panic!("Unknown BlockType value {}", x),
    };
    index += 1;

    let proc_name_len = u32::from_be_bytes([code[index], code[index + 1], code[index + 2], code[index + 3]]) as usize;
    index += 4;

    let proc_name = String::from_utf8(code[index..index + proc_name_len].to_vec()).expect("Invalid UTF-8 sequence");
    index += proc_name_len;

    let arg_count = code[index] as usize;
    index += 1;

    let root_block = Block {
      quote,
      proc_name,
      args: Vec::with_capacity(arg_count),
    };

    // 返却用に、Moveされてないブロック
    let mut last_root_block: Option<Block> = None;

    // 引数タイプを読み取る
    let mut arg_types = Vec::new();
    for _ in 0..arg_count {
      let expand = match code[index] {
        x if x == ArgPlugType::Expand as u8 => true,
        x if x == ArgPlugType::Normal as u8 => false,
        x => panic!("Unknown ArgPlugType {}", x),
      };
      arg_types.push(expand);
      index += 1;
    }
    arg_types.reverse();

    // 子ブロックを再構築する
    stack.push((root_block, arg_count));
    while let Some((parent_block, remaining_args)) = stack.pop() {
      if remaining_args == 0 {
        // 親ブロックが引数をすべて処理したら上位ブロックに戻る
        if let Some((upper_block, _)) = stack.last_mut() {
          upper_block.args.push((arg_types.pop().unwrap(), Box::new(parent_block)));
        } else {
          // 空なら今のブロックを返却用に束縛
          last_root_block = Some(parent_block);
        }
        continue;
      }

      // 引数の処理をする
      let quote = match code[index] {
        x if x == BlockType::Normal as u8 => QuoteStyle::None,
        x if x == BlockType::Quote as u8 => QuoteStyle::Quote,
        x if x == BlockType::Closure as u8 => QuoteStyle::Closure,
        x => panic!("Unknown BlockType [{}] = {}", index, x),
      };
      index += 1;

      let proc_name_len = u32::from_be_bytes([code[index], code[index + 1], code[index + 2], code[index + 3]]) as usize;
      index += 4;

      let proc_name = String::from_utf8(code[index..index + proc_name_len].to_vec()).expect("Invalid UTF-8 sequence");
      index += proc_name_len;

      let arg_count = code[index] as usize;
      index += 1;

      let child_block = Block {
        quote,
        proc_name,
        args: Vec::with_capacity(arg_count),
      };

      // 引数タイプを読み取る
      let mut tmp_arg_types = Vec::with_capacity(arg_count);
      for _ in 0..arg_count {
        let expand = match code[index] {
          x if x == ArgPlugType::Expand as u8 => true,
          x if x == ArgPlugType::Normal as u8 => false,
          x => panic!("Unknown ArgPlugType {}", x),
        };
        tmp_arg_types.push(expand);
        index += 1;
      }
      arg_types.extend(tmp_arg_types.iter().rev());

      // 引数が残っている場合は再度スタックに積む
      stack.push((parent_block, remaining_args - 1));

      // 子ブロックもスタックに積む
      if arg_count > 0 {
        stack.push((child_block, arg_count));
      } else {
        // 引数がない場合、直ちに親に追加する
        stack.last_mut().unwrap().0.args.push((arg_types.pop().unwrap(), Box::new(child_block)));
      }
    }

    last_root_block.unwrap()
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
  fn intermediate_to_one_block() {
    let block = Block::from_intermed_repr(&[1, 0, 0, 0, 4, 97, 97, 97, 97, 0]);

    assert_eq!(
      block,
      Block {
        quote: QuoteStyle::None,
        proc_name: "aaaa".into(),
        args: vec![],
      }
    );
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
        1, 0, 0, 0, 1, 97, 2, 1, 0, // type="normal",  name_len=1, name="a", child=2, @, not @,
        1, 0, 0, 0, 1, 98, 1, 0, // type="normal", name_len=1, name="b", child=1, not @,
        2, 0, 0, 0, 1, 99, 0, //  type="quote", name_len=1, name="c",  child=0,
        3, 0, 0, 0, 1, 100, 0, // type="closure",  name_len=1, name="d", child=0,
      ]
    );
  }

  #[test]
  fn intermediate_to_nest_block() {
    let block = Block::from_intermed_repr(&[
      1, 0, 0, 0, 1, 97, 2, 1, 0, // type="normal",  name_len=1, name="a", child=2, @, not @,
      1, 0, 0, 0, 1, 98, 1, 0, // type="normal", name_len=1, name="b", child=1, not @,
      2, 0, 0, 0, 1, 99, 0, //  type="quote", name_len=1, name="c",  child=0,
      3, 0, 0, 0, 1, 100, 0, // type="closure",  name_len=1, name="d", child=0,
    ]);

    assert_eq!(
      block,
      Block {
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
      }
    );
  }
}
