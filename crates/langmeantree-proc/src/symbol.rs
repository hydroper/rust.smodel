use crate::*;

pub struct Arena<T> {
    data: RefCell<Vec<Rc<T>>>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self {
            data: RefCell::new(vec![]),
        }
    }

    pub fn allocate(&self, value: T) -> Weak<T> {
        let obj = Rc::new(value);
        self.data.borrow_mut().push(obj.clone());
        Rc::downgrade(&obj)
    }
}

pub struct LmtFactory {
    arena: Arena<Symbol1>,
}

impl LmtFactory {
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
        }
    }

    pub fn create_meaning_slot(&self, name: String) -> Symbol {
        Symbol(self.arena.allocate(Symbol1::MeaningSlot(Rc::new(MeaningSlot1 {
            name,
            inherits: RefCell::new(None),
            submeanings: shared_array![],
            methods: shared_map![],
        }))))
    }

    pub fn create_field_slot(&self, is_ref: bool, name: String, field_type: syn::Type, field_init: syn::Expr) -> Symbol {
        Symbol(self.arena.allocate(Symbol1::FieldSlot(Rc::new(FieldSlot1 {
            is_ref,
            name,
            field_type,
            field_init,
        }))))
    }

    pub fn create_method_slot(&self, name: String, defined_in: Symbol, doc_attribute: Option<syn::Attribute>) -> Symbol {
        Symbol(self.arena.allocate(Symbol1::MethodSlot(Rc::new(MethodSlot1 {
            name,
            defined_in,
            doc_attribute,
            override_logic_mapping: SharedMap::new(),
        }))))
    }
}

#[derive(Clone)]
pub struct Symbol(Weak<Symbol1>);

impl Eq for Symbol {}

impl PartialEq for Symbol {
    fn eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}

impl Hash for Symbol {
    /// Performs hashing of the symbol by reference.
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.as_ptr().hash(state)
    }
}

macro access {
    ($symbol:expr) => { $symbol.0.upgrade().unwrap().as_ref() },
}

impl Symbol {
    pub fn is_meaning_slot(&self) -> bool {
        matches!(access!(self), Symbol1::MeaningSlot(_))
    }

    pub fn is_field_slot(&self) -> bool {
        matches!(access!(self), Symbol1::FieldSlot(_))
    }

    pub fn is_method_slot(&self) -> bool {
        matches!(access!(self), Symbol1::MethodSlot(_))
    }

    pub fn name(&self) -> String {
        match access!(self) {
            Symbol1::MeaningSlot(slot) => slot.name.clone(),
            Symbol1::FieldSlot(slot) => slot.name.clone(),
            Symbol1::MethodSlot(slot) => slot.name.clone(),
            _ => panic!(),
        }
    }

    pub fn inherits(&self) -> Option<Symbol> {
        match access!(self) {
            Symbol1::MeaningSlot(slot) => slot.inherits.borrow().clone(),
            _ => panic!(),
        }
    }

    pub fn set_inherits(&self, value: Option<&Symbol>) {
        match access!(self) {
            Symbol1::MeaningSlot(slot) => {
                slot.inherits.replace(value.map(|v| v.clone()));
            },
            _ => panic!(),
        }
    }

    pub fn submeanings(&self) -> SharedArray<Symbol> {
        match access!(self) {
            Symbol1::MeaningSlot(slot) => slot.submeanings.clone(),
            _ => panic!(),
        }
    }

    pub fn methods(&self) -> SharedMap<String, Symbol> {
        match access!(self) {
            Symbol1::MeaningSlot(slot) => slot.methods.clone(),
            _ => panic!(),
        }
    }

    pub fn field_type(&self) -> syn::Type {
        match access!(self) {
            Symbol1::FieldSlot(slot) => slot.field_type.clone(),
            _ => panic!(),
        }
    }

    pub fn field_init(&self) -> syn::Expr {
        match access!(self) {
            Symbol1::FieldSlot(slot) => slot.field_init.clone(),
            _ => panic!(),
        }
    }

    pub fn is_ref(&self) -> bool {
        match access!(self) {
            Symbol1::FieldSlot(slot) => slot.is_ref.clone(),
            _ => panic!(),
        }
    }

    pub fn defined_in(&self) -> Symbol {
        match access!(self) {
            Symbol1::MethodSlot(slot) => slot.defined_in.clone(),
            _ => panic!(),
        }
    }

    pub fn doc_attribute(&self) -> Option<syn::Attribute> {
        match access!(self) {
            Symbol1::MethodSlot(slot) => slot.doc_attribute.clone(),
            _ => panic!(),
        }
    }

    pub fn override_logic_mapping(&self) -> SharedMap<Symbol, Rc<OverrideLogicMapping>> {
        match access!(self) {
            Symbol1::MethodSlot(slot) => slot.override_logic_mapping.clone(),
            _ => panic!(),
        }
    }
}

impl ToString for Symbol {
    fn to_string(&self) -> String {
        self.name()
    }
}

enum Symbol1 {
    MeaningSlot(Rc<MeaningSlot1>),
    FieldSlot(Rc<FieldSlot1>),
    MethodSlot(Rc<MethodSlot1>),
}

struct MeaningSlot1 {
    name: String,
    inherits: RefCell<Option<Symbol>>,
    submeanings: SharedArray<Symbol>,
    methods: SharedMap<String, Symbol>,
}

struct FieldSlot1 {
    name: String,
    field_type: syn::Type,
    field_init: syn::Expr,
    is_ref: bool,
}

struct MethodSlot1 {
    name: String,
    defined_in: Symbol,
    doc_attribute: Option<syn::Attribute>,
    override_logic_mapping: SharedMap<Symbol, Rc<OverrideLogicMapping>>,
}

pub struct OverrideLogicMapping {
    override_code: RefCell<Option<proc_macro2::TokenTree>>,
    override_logic_mapping: SharedMap<Symbol, Rc<OverrideLogicMapping>>,
}

impl OverrideLogicMapping {
    pub fn new() -> Self {
        Self {
            override_code: RefCell::new(None),
            override_logic_mapping: SharedMap::new(),
        }
    }

    /// Override code; generally a `return` statement with a semicolon.
    pub fn override_code(&self) -> Option<proc_macro2::TokenTree> {
        self.override_code.borrow().clone()
    }

    /// Sets override code; generally a `return` statement with a semicolon.
    pub fn set_override_code(&self, code: Option<proc_macro2::TokenTree>) {
        self.override_code.replace(code);
    }

    /// Mapping from submeaning slot to override logic.
    pub fn override_logic_mapping(&self) -> SharedMap<Symbol, Rc<OverrideLogicMapping>> {
        self.override_logic_mapping.clone()
    }
}

/// A meaning slot.
/// 
/// # Supported methods
/// 
/// * `is_meaning_slot()` — Returns `true`.
/// * `name()`
/// * `inherits()`
/// * `set_inherits()`
/// * `submeanings()`
/// * `methods()`
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct MeaningSlot(pub Symbol);

impl Deref for MeaningSlot {
    type Target = Symbol;
    fn deref(&self) -> &Self::Target {
        assert!(self.0.is_meaning_slot());
        &self.0
    }
}

/// A field slot.
/// 
/// # Supported methods
/// 
/// * `is_field_slot()` — Returns `true`.
/// * `is_ref()`
/// * `name()`
/// * `field_type()`
/// * `field_init()`
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct FieldSlot(pub Symbol);

impl Deref for FieldSlot {
    type Target = Symbol;
    fn deref(&self) -> &Self::Target {
        assert!(self.0.is_field_slot());
        &self.0
    }
}

/// A method slot.
/// 
/// # Supported methods
/// 
/// * `is_method_slot()` — Returns `true`.
/// * `name()`
/// * `defined_in()`
/// * `doc_attribute()`
/// * `override_logic_mapping()` — Mapping from submeaning slot to override logic.
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct MethodSlot(pub Symbol);

impl Deref for MethodSlot {
    type Target = Symbol;
    fn deref(&self) -> &Self::Target {
        assert!(self.0.is_method_slot());
        &self.0
    }
}