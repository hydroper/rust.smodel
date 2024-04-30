use crate::*;

const CTOR_INIT_NAME: &'static str = "__ctor";

pub struct ProcessingStep3_7();

impl ProcessingStep3_7 {
    // Define the constructor
    pub fn exec(&self, _host: &mut SModelHost, node: Option<&MeaningConstructor>, meaning: &Symbol, asc_meaning_list: &[Symbol], arena_type_name: &str) {
        let input = node.map(|node| node.inputs.clone()).unwrap_or(Punctuated::new());
        let type_params = node.map(|node| [node.generics.lt_token.to_token_stream(), node.generics.params.to_token_stream(), node.generics.gt_token.to_token_stream()]).unwrap_or([
            proc_macro2::TokenStream::new(),
            proc_macro2::TokenStream::new(),
            proc_macro2::TokenStream::new(),
        ]);
        let where_clause = node.map(|node| node.generics.where_clause.as_ref().map(|c| c.to_token_stream()).unwrap_or(proc_macro2::TokenStream::new())).unwrap_or(proc_macro2::TokenStream::new());
        let attr = node.map(|node| node.attributes.clone()).unwrap_or(vec![]);
        let vis = node.map(|node| node.visibility.to_token_stream()).unwrap_or(proc_macro2::TokenStream::new());

        let ctor_init_name_id = Ident::new(CTOR_INIT_NAME, Span::call_site());
        let arena_type_name_id = Ident::new(arena_type_name, Span::call_site());

        // Define the the instance `#ctor_init_name_id` method,
        // containing everything but `super()` and structure initialization.
        let statements = node.map(|node| node.statements.clone()).unwrap_or(vec![]);
        meaning.method_output().borrow_mut().extend(quote! {
            #(#attr)*
            fn #ctor_init_name_id #(#type_params)*(&self, #input) #where_clause {
                #(#statements)*
            }
        });

        // `M::new` output
        let mut m_new_out = TokenStream::new();

        // At `M::new`, let `__cto1` be a complex `M2(M1(__arena.allocate(#DATA::M1 { ... })))`
        // (notice the meaning layers) allocation initializing all meaning variants's fields
        // with their default values.
        let initlayer1 = self.init_data(asc_meaning_list, 0);
        let initlayer2 = proc_macro2::TokenStream::from_str(&Symbol::create_layers_over_weak_root(&format!("arena.allocate({})", initlayer1.to_string()), asc_meaning_list)).unwrap();
        m_new_out.extend::<TokenStream>(quote! {
            let __cto1 = #initlayer2;
        }.try_into().unwrap());

        // If the meaning inherits another meaning:
        //
        // * At `M::new`, invoke `InheritedM::#ctor_init_name_id(&__cto1.0, ...super_arguments)`,
        //   passing all `super(...)` arguments.
        if let Some(inherited_m) = meaning.inherits() {
            let inherited_m_name = Ident::new(&inherited_m.name(), Span::call_site());
            let super_arguments = node.map(|node| node.super_arguments.clone()).unwrap_or(Punctuated::new());
            m_new_out.extend::<TokenStream>(quote! {
                #inherited_m_name::#ctor_init_name_id(&__cto1.0, #super_arguments);
            }.try_into().unwrap());
        }

        // * Output a `__cto1.#ctor_init_name_id(...arguments);` call to `M::new`.
        // * Output a `__cto1` return to `M::new`.
        let input_args = convert_function_input_to_arguments(&input);
        m_new_out.extend::<TokenStream>(quote! {
            __cto1.#ctor_init_name_id(#input_args);
            __cto1
        }.try_into().unwrap());

        // Output the constructor as a static `new` method (`M::new`) with
        // a prepended `arena: &#arena_type_name_id` parameter.

        let m_new_out: proc_macro2::TokenStream = m_new_out.into();

        meaning.method_output().borrow_mut().extend(quote! {
            #(#attr)*
            #vis fn new #(#type_params)*(arena: &#arena_type_name_id, #input) -> Self #where_clause {
                #m_new_out
            }
        });
    }

    fn init_data(&self, asc_meaning_list: &[Symbol], meaning_index: usize) -> proc_macro2::TokenStream {
        let meaning = &asc_meaning_list[meaning_index];
        let meaning_name = meaning.name();
        let mut fields = proc_macro2::TokenStream::new();
        for (name, field) in meaning.fields().borrow().iter() {
            let name_id = Ident::new(name, Span::call_site());
            let fv = field.field_init();
            if field.is_ref() {
                fields.extend(quote! {
                    #name_id: ::std::cell::RefCell::new(#fv),
                });
            } else {
                fields.extend(quote! {
                    #name_id: ::std::cell::Cell::new(#fv),
                });
            }
        }
        let data_variant_no_submeaning = proc_macro2::TokenStream::from_str(DATA_VARIANT_NO_SUBMEANING).unwrap();
        let submeaning_enum = proc_macro2::TokenStream::from_str(&format!("{DATA}::{DATA_VARIANT_PREFIX}{meaning_name}")).unwrap();
        let variant = if meaning_index + 1 < asc_meaning_list.len() {
            let next_m = asc_meaning_list[meaning_index + 1].name();
            let next_m = Ident::new(&next_m, Span::call_site());
            let i = self.init_data(asc_meaning_list, meaning_index + 1);
            quote! { #submeaning_enum::#next_m(::std::rc::Rc::new(#i)) }
        } else {
            quote! { #submeaning_enum::#data_variant_no_submeaning }
        };
        let data_variant_field = Ident::new(DATA_VARIANT_FIELD, Span::call_site());
        let data_id = Ident::new(DATA, Span::call_site());
        let meaning_name_id = Ident::new(&meaning_name, Span::call_site());
        quote! {
            #data_id::#meaning_name_id {
                #fields
                #data_variant_field: #variant
            }
        }
    }
}