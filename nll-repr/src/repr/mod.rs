use intern::InternedString;
use lalrpop_util::ParseError;
use std::collections::BTreeMap;
use std::iter;

mod parser;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct BasicBlock(pub InternedString);

#[derive(Clone, Debug)]
pub struct Func {
    pub decls: Vec<VariableDecl>,
    pub structs: Vec<StructDecl>,
    pub data: BTreeMap<BasicBlock, BasicBlockData>,
    pub assertions: Vec<Assertion>
}

impl Func {
    pub fn parse(s: &str) -> Result<Self, String> {
        let err_loc = match parser::parse_Func(s) {
            Ok(f) => return Ok(f),
            Err(ParseError::InvalidToken { location }) => location,
            Err(ParseError::UnrecognizedToken { token: None, .. }) => s.len(),
            Err(ParseError::UnrecognizedToken { token: Some((l, _, _)), .. }) => l,
            Err(ParseError::ExtraToken { token: (l, _, _) }) => l,
            Err(ParseError::User { .. }) => unimplemented!()
        };

        let line_num = s[..err_loc].lines().count();
        let col_num = s[..err_loc].lines().last().map(|s| s.len()).unwrap_or(0);
        Err(format!("parse error at {}:{} (offset {})", line_num, col_num + 1, err_loc))
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct StructDecl {
    pub name: StructName,
    pub parameters: Vec<StructParameter>,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct StructParameter {
    pub kind: Kind,
    pub variance: Variance,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Kind {
    Region,
    Type,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Variance {
    Co,
    Contra,
    In,
}

impl Variance {
    pub fn invert(self) -> Variance {
        match self {
            Variance::Co => Variance::Contra,
            Variance::Contra => Variance::Co,
            Variance::In => Variance::In,
        }
    }

    pub fn xform(self, v: Variance) -> Variance {
        match self {
            Variance::Co => v,
            Variance::Contra => v.invert(),
            Variance::In => Variance::In,
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct StructName {
    name: InternedString
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Ty {
    Ref(RegionName, Box<Ty>),
    RefMut(RegionName, Box<Ty>),
    Unit,
    Struct(StructName, Vec<TyParameter>),
}

impl Ty {
    pub fn walk<'a>(&'a self, v: Variance) -> Box<Iterator<Item = (Variance, RegionName)> + 'a> {
        match *self {
            Ty::Ref(rn, ref t) => Box::new(
                iter::once((v, rn))
                    .chain(t.walk(v))
            ),
            Ty::RefMut(rn, ref t) => Box::new(
                iter::once((v, rn))
                    .chain(t.walk(v.xform(Variance::In)))
            ),
            Ty::Unit => Box::new(
                iter::empty()
            ),
            Ty::Struct(_, ref params) => Box::new(
                params.iter()
                      .flat_map(move |p| match *p {
                          TyParameter::Region(rn) => Box::new(iter::once((v, rn))),
                          TyParameter::Ty(ref t) => t.walk(v),
                      })
            ),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum TyParameter {
    Region(RegionName),
    Ty(Box<Ty>),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct BasicBlockData {
    pub name: BasicBlock,
    pub actions: Vec<Action>,
    pub successors: Vec<BasicBlock>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Action {
    Borrow(Variable, RegionName), // p = &'X
    Assign(Variable, Variable), // p = q;
    Constraint(Box<Constraint>), // C
    Use(Variable), // use(p);
    Write(Variable), // write(p);
    Noop,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Constraint {
    ForAll(Vec<RegionName>, Box<Constraint>),
    Exists(Vec<RegionName>, Box<Constraint>),
    Implies(Vec<OutlivesConstraint>, Box<Constraint>),
    All(Vec<Constraint>),
    Outlives(OutlivesConstraint),
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct OutlivesConstraint {
    pub sup: RegionName,
    pub sub: RegionName,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Variable {
    name: InternedString,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct VariableDecl {
    pub var: Variable,
    pub ty: Box<Ty>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Assertion {
    Eq(RegionName, Region),
    In(RegionName, Point),
    NotIn(RegionName, Point),
    Live(Variable, BasicBlock),
    NotLive(Variable, BasicBlock),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Point {
    pub block: BasicBlock,
    pub action: usize,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct RegionName {
    name: InternedString
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Region {
    pub points: Vec<Point>,
}
