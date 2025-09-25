use alloc::{vec, vec::Vec};

use crate::{parser::{self, Ast}, span::Spanned, CompileError};

type TypeId = usize;

enum Type {
    Unit,
    Bool,
    Integer {
        log2_size: u8,
        signed: bool,
    },
    Function {
        params: Vec<TypeId>,
        ret: TypeId,
    }
}

pub struct Checker {
    types: Vec<Type>,
}

impl Checker {
    pub fn check<'src>(ast: &Ast<'src>) -> Result<Self, Spanned<'src, CompileError>> {
        let checker = Self { types: vec![
            Type::Unit,
            Type::Bool,
            Type::Integer { log2_size: 1, signed: true },
            Type::Integer { log2_size: 1, signed: false },
            Type::Integer { log2_size: 2, signed: true },
            Type::Integer { log2_size: 2, signed: false },
            Type::Integer { log2_size: 3, signed: true },
            Type::Integer { log2_size: 3, signed: false },
            Type::Integer { log2_size: 4, signed: true },
            Type::Integer { log2_size: 4, signed: false },
        ]};

        todo!("{:?}", ast);
    }

    pub fn get_or_insert(&mut self, ty: &parser::Type) -> TypeId {
        match ty {
            parser::Type::Simple(name) => {
                match *name {
                    "bool" => return 0,
                    "i8" => return 1,
                    "u8" => return 2,
                    "i16" => return 3,
                    "u16" => return 4,
                    "i32" => return 5,
                    "u32" => return 6,
                    "i64" => return 7,
                    "u64" => return 8,
                    _ => (),
                };

                //TODO: LATER CHECK CUSTOM TYPES
                todo!("custom types")
            },
            parser::Type::Function(items, ret) => {
                let oty = self.types.iter().enumerate().find(|(_, oty)| {
                    let oret = ret;
                    match oty {
                        Type::Function { params, ret } => todo!(),
                        _ => false,
                    }
                });

                match oty {
                    Some((oty, _)) => oty,
                    None => todo!(),
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{checker::Checker, parser::Ast, CompileError, span::Spanned};

    #[test]
    fn test_checker() -> Result<(), Spanned<'static, CompileError>> {
        const SOURCE: &str = "const a |= false;";

        let ast = Ast::parse(SOURCE)?;

        Checker::check(&ast)?;

        Ok(())
    }
}
