use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{
    parse_macro_input, DeriveInput, Field, GenericArgument, ItemStruct, Path, PathArguments, Type,
    TypePath,
};

fn field_parse(field: &Field) -> proc_macro2::TokenStream {
    match &field.ty {
        Type::Path(TypePath { path, .. }) if path.segments.last().unwrap().ident == "Vec" => {
            let ty = get_generic(path).unwrap();

            match ty {
                Type::Path(TypePath { path, .. }) if path.is_ident("u8") => {
                    quote!({
                        let current_pos = mp4.reader.buffer.stream_position()?;
                        mp4.reader
                            .read_bytes_dyn((offset + len).saturating_sub(current_pos) as usize)?
                    })
                }
                _ => {
                    quote!(
                        {
                            let mut v = Vec::with_capacity((offset + len - mp4.reader.buffer.stream_position()?) as usize / std::mem::size_of::<#ty>());
                            while (offset + len - mp4.reader.buffer.stream_position()?) > 0 {
                                v.push(<#ty as crate::Parse>::parse(mp4)?);
                            }
                            v
                        }
                    )
                }
            }
        }
        Type::Path(TypePath { path, .. }) if path.segments.last().unwrap().ident == "String" => {
            quote!({
                let current_pos = mp4.reader.buffer.stream_position()?;
                String::from_utf8(
                    mp4.reader
                        .read_bytes_dyn((offset + len).saturating_sub(current_pos) as usize)?,
                )
                .unwrap()
            })
        }
        _ => {
            let ty = &field.ty;
            quote!(<#ty as crate::Parse>::parse(mp4)?)
        }
    }
}

#[proc_macro_attribute]
pub fn mp4_atom(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let x = item.clone();
    let input = parse_macro_input!(item as DeriveInput);
    let item_struct = parse_macro_input!(x as ItemStruct);

    let struct_field_names = item_struct
        .fields
        .iter()
        .map(|f| &f.ident)
        .collect::<Vec<_>>();

    let struct_field_parse = item_struct.fields.iter().map(field_parse);

    let name = item_struct.ident;

    quote!(
        #[derive(Debug, Clone)]
        #input

        impl crate::Parse for #name {
            fn parse<R: std::io::BufRead + std::io::Seek>(mp4: &mut crate::Mp4<'_, R>) -> std::io::Result<Self>
                where Self: Sized {
                let offset = mp4.reader.buffer.stream_position()?;
                let len = mp4.read_atom_len()?;
                mp4.expect_header(Self::HEADER)?;

                #(
                    let #struct_field_names = #struct_field_parse;
                )*

                if offset != mp4.reader.buffer.stream_position()?.saturating_sub(len as u64) {
                    dbg!(#(
                        &#struct_field_names,
                    )*);
                }

                assert_eq!(offset, mp4.reader.buffer.stream_position()?.saturating_sub(len as u64));

                Ok(Self {
                    #(
                        #struct_field_names,
                    )*
                })
            }
        }
    ).into()
}

fn get_generic(path: &Path) -> Option<&Type> {
    match &path.segments.last().unwrap().arguments {
        PathArguments::AngleBracketed(generic) => match &generic.args.last() {
            Some(GenericArgument::Type(ty)) => Some(ty),
            _ => None,
        },
        _ => None,
    }
}

#[proc_macro_attribute]
pub fn mp4_container_atom(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_struct = parse_macro_input!(item as ItemStruct);

    let vis = &item_struct.vis;
    let struct_name = &item_struct.ident;

    let internal_name = &Ident::new(&(struct_name.to_string() + "__internal"), Span::call_site());

    let struct_field_names = item_struct
        .fields
        .iter()
        .map(|f| &f.ident)
        .collect::<Vec<_>>();
    let struct_field_types = item_struct.fields.iter().map(|f| &f.ty).collect::<Vec<_>>();
    // let struct_field_vis = item_struct.fields.iter().map(|f| &f.vis);

    let struct_field_search = item_struct.fields.iter().map(|field| match &field.ty {
        Type::Path(TypePath { path, .. }) if path.segments.last().unwrap().ident == "Vec" => {
            let generic = get_generic(path).unwrap();
            let ref_generic = get_generic(if let Type::Path(TypePath { path, .. }) = generic {
                path
            } else {
                panic!()
            })
            .unwrap();

            quote!(
                self.unparsed_atoms.drain_filter(|atom|{
                    mp4.jump_to(atom.offset).unwrap();
                    mp4.peek_header().unwrap() == <#ref_generic>::HEADER
                }).map(|atom| atom.into_ref::<#ref_generic>()).collect()
            )
        }
        Type::Path(TypePath { path, .. }) if path.segments.last().unwrap().ident == "Option" => {
            let generic = get_generic(path).unwrap();
            let ref_generic = get_generic(if let Type::Path(TypePath { path, .. }) = generic {
                path
            } else {
                panic!()
            })
            .unwrap();

            quote!(
                self.unparsed_atoms.drain_filter(|atom|{
                    mp4.jump_to(atom.offset).unwrap();
                    mp4.peek_header().unwrap() == <#ref_generic>::HEADER
                }).map(|atom| atom.into_ref::<#ref_generic>()).next()
            )
        }
        Type::Path(TypePath { path, .. }) if path.segments.last().unwrap().ident == "Reference" => {
            let generic = get_generic(path).unwrap();
            quote!(
                self.unparsed_atoms.drain_filter(|atom|{
                    mp4.jump_to(atom.offset).unwrap();
                    mp4.peek_header().unwrap() == <#generic>::HEADER
                }).map(|atom| atom.into_ref::<#generic>()).next().unwrap()
            )
        }
        _ => panic!("expected reference, vec, or option"),
    });

    quote!(
        #[derive(Debug, Clone)]
        struct #internal_name {
            #(
                #struct_field_names: InternalElement<#struct_field_types>,
            )*
        }

        #[derive(Debug, Clone)]
        #vis struct #struct_name {
            unparsed_atoms: Vec<UnparsedAtom>,
            __internal: #internal_name,
        }

        impl #struct_name {
            #(
                pub fn #struct_field_names<R: std::io::BufRead + std::io::Seek>(&mut self, mp4: &mut Mp4<'_, R>) -> &#struct_field_types {
                    match self.__internal.#struct_field_names {
                        InternalElement::Searched(ref v) => v,
                        InternalElement::NotSearched => {
                            self.__internal.#struct_field_names = InternalElement::Searched(#struct_field_search);
                            self.#struct_field_names(mp4)
                        },
                    }
                }
            )*
        }

        impl crate::Parse for #struct_name {
            fn parse<R: std::io::BufRead + std::io::Seek>(mp4: &mut crate::Mp4<'_, R>) -> std::io::Result<Self>
                    where Self: Sized {
                let offset = mp4.reader.buffer.stream_position()?;
                let len = mp4.read_atom_len()?;
                let header = Header(mp4.reader.read_bytes_const::<4>()?);

                let mut unparsed_atoms = Vec::new();

                while offset + len - mp4.reader.buffer.stream_position()? > 0 {
                    unparsed_atoms.push(UnparsedAtom::parse(mp4)?);
                }

                Ok(Self {
                    unparsed_atoms,
                    __internal: #internal_name {
                        #(
                            #struct_field_names: InternalElement::NotSearched,
                        )*
                    }
                })
            }
        }
    ).into()
}

#[proc_macro_attribute]
pub fn mp4_media_data_type_atom(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let x = item.clone();
    let input = parse_macro_input!(item as DeriveInput);
    let item_struct = parse_macro_input!(x as ItemStruct);

    let struct_field_names = item_struct
        .fields
        .iter()
        .map(|f| &f.ident)
        .collect::<Vec<_>>();

    let struct_field_parse = item_struct.fields.iter().map(field_parse);

    let name = item_struct.ident;

    quote!(
        #[derive(Debug, Clone)]
        #input

        impl crate::Parse for #name {
            fn parse<R: std::io::BufRead + std::io::Seek>(mp4: &mut crate::Mp4<'_, R>) -> std::io::Result<Self>
                where Self: Sized {
                let offset = mp4.reader.buffer.stream_position()?;
                let len = mp4.reader.read_u32()? as u64;

                #(
                    let #struct_field_names = #struct_field_parse;
                )*

                assert_eq!(offset, mp4.reader.buffer.stream_position()?.saturating_sub(len as u64));

                Ok(Self {
                    #(
                        #struct_field_names,
                    )*
                })
            }
        }
    ).into()
}
