use anyhow::Result;
use wasm_encoder::{
    CodeSection, DataSection, DataSegment, DataSegmentMode, ElementSection, EntityType, ExportKind,
    ExportSection, Function, FunctionSection, GlobalSection, GlobalType, ImportSection,
    Instruction, MemorySection, MemoryType, Module, StartSection, TableSection, TableType,
    TypeSection, ValType,
};
use wasmparser::Payload;

use crate::wasm_translate::translator::*;

pub type Params = Vec<ValType>;
pub type Results = Vec<ValType>;

#[derive(Debug)]
pub struct Signature {
    pub params: Params,
    pub results: Results,
}

#[derive(Debug)]
pub struct Import {
    module: String,
    field: String,
    ty: EntityType,
}

pub type TypeIndex = u32;

#[derive(Debug)]
pub struct Glob<'a> {
    pub global_type: GlobalType,
    pub init_instr: Instruction<'a>,
}

#[derive(Debug)]
pub struct Export {
    pub name: String,
    pub kind: ExportKind,
    pub index: u32,
}

#[derive(Debug)]
pub struct ModuleBuilder<'a> {
    pub types: Vec<Signature>,
    pub imports: Vec<Import>,
    pub functions: Vec<TypeIndex>,
    pub tables: Vec<TableType>,
    pub memories: Vec<MemoryType>,
    pub globals: Vec<Glob<'a>>,
    pub exports: Vec<Export>,
    pub start_sect: Option<StartSection>,
    pub elements: Vec<translator::ElementSegment>,
    pub code: Vec<Function>,
    pub data: Vec<Data<'a>>,
}

impl<'a> ModuleBuilder<'a> {
    fn new() -> Self {
        ModuleBuilder {
            types: Default::default(),
            imports: Default::default(),
            functions: Default::default(),
            tables: Default::default(),
            memories: Default::default(),
            globals: Default::default(),
            exports: Default::default(),
            start_sect: None,
            elements: Default::default(),
            code: Default::default(),
            data: Default::default(),
        }
    }

    pub fn add_type(&mut self, sig: Signature) -> TypeIndex {
        self.types.push(sig);
        u32::try_from(self.types.len()).unwrap() - 1
    }

    pub fn add_function(&mut self, sig: Signature, body: Function) -> u32 {
        let type_idx = self.add_type(sig);
        self.functions.push(type_idx);
        let sig_id = u32::try_from(self.functions.len()).unwrap() - 1;
        self.code.push(body);
        let code_id = u32::try_from(self.code.len()).unwrap() - 1;
        assert_eq!(sig_id, code_id);
        sig_id
    }

    pub fn add_export(&mut self, export: Export) -> u32 {
        self.exports.push(export);
        u32::try_from(self.exports.len()).unwrap() - 1
    }

    pub fn build(self) -> Module {
        let mut m = Module::new();
        let mut type_section = TypeSection::new();
        for Signature { params, results } in self.types {
            type_section.function(params, results);
        }
        m.section(&type_section);

        let mut import_section = ImportSection::new();
        for i in self.imports {
            import_section.import(&i.module, &i.field, i.ty);
        }
        m.section(&import_section);

        let mut function_section = FunctionSection::new();
        for f in self.functions {
            function_section.function(f);
        }
        m.section(&function_section);

        let mut table_section = TableSection::new();
        for t in self.tables {
            table_section.table(t);
        }
        m.section(&table_section);

        let mut memory_section = MemorySection::new();
        for m in self.memories {
            memory_section.memory(m);
        }
        m.section(&memory_section);

        let mut global_section = GlobalSection::new();
        for global in self.globals {
            let init = constexpr_from_instr(global.init_instr);
            global_section.global(global.global_type, &init);
        }
        m.section(&global_section);

        let mut export_section = ExportSection::new();
        for e in self.exports {
            export_section.export(&e.name, e.kind, e.index);
        }
        m.section(&export_section);

        if let Some(start_sect) = self.start_sect {
            m.section(&start_sect);
        }

        let mut element_section = ElementSection::new();
        for e in self.elements {
            let mut modified = e;
            if let translator::ElementMode::Active {
                ref mut table,
                offset: _,
            } = modified.mode
            {
                *table = None; // seems like near runtime does not support multiple tables. at least it definitely does not respect any table indexes there
            }
            element_section.segment(modified.borrowed());
        }
        m.section(&element_section);

        let mut code_section = CodeSection::new();
        for c in self.code {
            code_section.function(&c);
        }
        m.section(&code_section);

        let mut data_section = DataSection::new();
        let mut const_exprs = vec![];
        for d in self.data {
            let mode = match d.mode {
                DataMode::Active {
                    memory_index,
                    offset_instr,
                } => {
                    let offset_constexpr = constexpr_from_instr(offset_instr);
                    const_exprs.push(offset_constexpr);
                    DataSegmentMode::Active {
                        memory_index,
                        offset: const_exprs.last().unwrap(),
                    }
                }
                DataMode::Passive => DataSegmentMode::Passive,
            };
            let data_segment = DataSegment { mode, data: d.data };
            data_section.segment(data_segment);
        }
        m.section(&data_section);

        m
    }
}

pub fn parse(wasm: &Vec<u8>) -> Result<ModuleBuilder> {
    let parsed = wasmparser::Parser::new(0)
        .parse_all(wasm.as_slice())
        .map(|p| p.unwrap())
        .collect::<Vec<_>>();

    let mut code_section_size: Option<u32> = None;

    let mut builder = ModuleBuilder::new();
    for p in parsed {
        match p {
            Payload::Version {
                num: _version,
                encoding,
                range: _range,
            } => {
                assert_eq!(encoding, wasmparser::Encoding::Module);
            }
            Payload::TypeSection(type_section) => {
                for typ in type_section {
                    let (params, results) = type_def(typ?)?;
                    builder.types.push(Signature { params, results });
                }
            }
            Payload::ImportSection(import_section) => {
                for import in import_section {
                    let import = import?;
                    let typ = type_ref(import.ty)?;
                    builder.imports.push(Import {
                        module: import.module.to_string(),
                        field: import.name.to_string(),
                        ty: typ,
                    });
                }
            }
            Payload::FunctionSection(function_section) => {
                for function in function_section {
                    builder.functions.push(function?);
                }
            }
            Payload::TableSection(table_section) => {
                for table in table_section {
                    let table = table?;
                    if let wasmparser::TableInit::RefNull = table.init {
                    } else {
                        panic!("unknown table init");
                    }
                    let table_type = table_type(&table.ty)?;
                    builder.tables.push(table_type);
                }
            }
            Payload::MemorySection(memory_section) => {
                for mem in memory_section {
                    let mem = mem?;
                    let memory_type = memory_type(&mem)?;
                    builder.memories.push(memory_type);
                }
            }
            Payload::TagSection(tag_section) => {
                for tag in tag_section {
                    let tag = tag?;
                    let _tag_type = tag_type(&tag)?;
                    panic!("only wasm core - 1 specification is supported by near runtime");
                    // https://github.com/near/nearcore/issues/8358#issuecomment-1383247423
                }
            }
            Payload::GlobalSection(global_section) => {
                for glob in global_section {
                    let (global_type, init_instr) = global(glob?)?;
                    builder.globals.push(Glob {
                        global_type,
                        init_instr,
                    });
                }
            }
            Payload::ExportSection(export_section) => {
                for export in export_section {
                    let export = export?;
                    builder.exports.push(Export {
                        name: export.name.to_string(),
                        kind: ext_kind(export.kind),
                        index: export.index,
                    });
                }
            }
            Payload::StartSection {
                func: function_index,
                range: _range,
            } => {
                let start_sect = StartSection { function_index };
                builder.start_sect = Some(start_sect);
            }
            Payload::ElementSection(element_section) => {
                for e in element_section {
                    let element_segment = element(e?)?;
                    builder.elements.push(element_segment);
                }
            }
            Payload::DataCountSection { count: _, range: _ } => {
                panic!("only wasm core - 1 specification is supported by near runtime");
                // https://github.com/near/nearcore/issues/8358#issuecomment-1383247423
            }
            Payload::DataSection(data_section) => {
                for d in data_section {
                    let data_seg = data(d?)?;
                    builder.data.push(data_seg);
                }
            }
            Payload::CodeSectionStart {
                count,
                range: _,
                size: _,
            } => {
                code_section_size = Some(count);
            }
            Payload::CodeSectionEntry(code_section_entry) => {
                assert!(code_section_size.is_some());
                let code_seg = code(code_section_entry)?;
                builder.code.push(code_seg);
            }
            Payload::ModuleSection {
                parser: _,
                range: _,
            } => unimplemented!("unsupported section (ModuleSection)"),
            Payload::InstanceSection(_) => unimplemented!("unsupported section (InstanceSection)"),
            Payload::CoreTypeSection(_) => unimplemented!("unsupported section (CoreTypeSection)"),
            Payload::ComponentSection {
                parser: _,
                range: _,
            } => unimplemented!("unsupported section (ComponentSection)"),
            Payload::ComponentInstanceSection(_) => {
                unimplemented!("unsupported section (ComponentInstanceSection)")
            }
            Payload::ComponentAliasSection(_) => {
                unimplemented!("unsupported section (ComponentAliasSection)")
            }
            Payload::ComponentTypeSection(_) => {
                unimplemented!("unsupported section (ComponentTypeSection)")
            }
            Payload::ComponentCanonicalSection(_) => {
                unimplemented!("unsupported section (ComponentCanonicalSection)")
            }
            Payload::ComponentStartSection { start: _, range: _ } => {
                unimplemented!("unsupported section (ComponentStartSection)")
            }
            Payload::ComponentImportSection(_) => {
                unimplemented!("unsupported section (ComponentImportSection)")
            }
            Payload::ComponentExportSection(_) => {
                unimplemented!("unsupported section (ComponentExportSection)")
            }
            Payload::CustomSection(_) => unimplemented!("unsupported section (CustomSection)"),
            Payload::UnknownSection {
                id: _,
                contents: _,
                range: _,
            } => unimplemented!("unsupported section (UnknownSection)"),
            Payload::End(_end) => {}
        }
    }

    assert_eq!(
        u32::try_from(builder.code.len()).unwrap(),
        code_section_size.unwrap()
    );

    Ok(builder)
}

pub(crate) mod translator {
    use anyhow::{Error, Result};
    use wasm_encoder::*;
    use wasmparser::{for_each_operator, ExternalKind, FunctionBody, Operator, Type};

    pub fn constexpr_from_instr(instr: Instruction) -> ConstExpr {
        let mut const_expr_buf = Vec::new();
        instr.encode(&mut const_expr_buf);
        ConstExpr::raw(const_expr_buf)
    }

    #[derive(Debug)]
    pub struct ElementSegment {
        pub mode: ElementMode,
        pub element_type: RefType,
        pub elements: Elements,
    }

    impl ElementSegment {
        pub fn borrowed(&self) -> wasm_encoder::ElementSegment {
            wasm_encoder::ElementSegment {
                mode: self.mode.borrowed(),
                element_type: self.element_type,
                elements: self.elements.borrowed(),
            }
        }
    }

    #[derive(Debug)]
    pub enum ElementMode {
        Passive,
        Declared,
        Active {
            table: Option<u32>,
            offset: ConstExpr,
        },
    }

    impl ElementMode {
        pub fn borrowed(&self) -> wasm_encoder::ElementMode {
            match self {
                Self::Passive => wasm_encoder::ElementMode::Passive,
                Self::Declared => wasm_encoder::ElementMode::Declared,
                Self::Active { table, offset } => wasm_encoder::ElementMode::Active {
                    table: *table,
                    offset,
                },
            }
        }
    }

    #[derive(Debug)]
    pub enum Elements {
        Functions(Vec<u32>),
        Expressions(Vec<wasm_encoder::ConstExpr>),
    }

    impl Elements {
        pub fn borrowed(&self) -> wasm_encoder::Elements {
            match self {
                Self::Functions(xs) => wasm_encoder::Elements::Functions(xs),
                Self::Expressions(xs) => wasm_encoder::Elements::Expressions(xs),
            }
        }
    }

    #[derive(Debug)]
    pub enum DataMode<'a> {
        Active {
            memory_index: u32,
            offset_instr: Instruction<'a>,
        },
        Passive,
    }

    #[derive(Debug)]
    pub struct Data<'a> {
        pub mode: DataMode<'a>,
        pub data: Vec<u8>,
    }

    pub fn type_ref(type_ref: wasmparser::TypeRef) -> Result<EntityType> {
        match type_ref {
            wasmparser::TypeRef::Func(f) => Ok(EntityType::Function(f)),
            wasmparser::TypeRef::Table(t) => {
                let element_type = refty(&t.element_type)?;
                Ok(EntityType::Table(TableType {
                    element_type,
                    minimum: t.initial,
                    maximum: t.maximum,
                }))
            }
            wasmparser::TypeRef::Memory(m) => Ok(EntityType::Memory(MemoryType {
                memory64: m.memory64,
                minimum: m.initial,
                maximum: m.maximum,
                shared: m.shared,
            })),
            wasmparser::TypeRef::Global(g) => Ok(EntityType::Global(GlobalType {
                val_type: ty(&g.content_type)?,
                mutable: g.mutable,
            })),
            wasmparser::TypeRef::Tag(t) => Ok(EntityType::Tag(tag_type(&t)?)),
        }
    }

    pub fn type_def(typ: Type) -> Result<(Vec<ValType>, Vec<ValType>)> {
        match typ {
            Type::Func(f) => {
                let params = f.params().iter().map(ty).collect::<Result<Vec<_>>>()?;
                let results = f.results().iter().map(ty).collect::<Result<Vec<_>>>()?;
                Ok((params, results))
            }
        }
    }

    pub fn ext_kind(ekind: ExternalKind) -> ExportKind {
        match ekind {
            ExternalKind::Func => ExportKind::Func,
            ExternalKind::Table => ExportKind::Table,
            ExternalKind::Memory => ExportKind::Memory,
            ExternalKind::Global => ExportKind::Global,
            ExternalKind::Tag => ExportKind::Tag,
        }
    }

    pub fn table_type(ty: &wasmparser::TableType) -> Result<TableType> {
        Ok(TableType {
            element_type: refty(&ty.element_type)?,
            minimum: ty.initial,
            maximum: ty.maximum,
        })
    }

    pub fn memory_type(ty: &wasmparser::MemoryType) -> Result<MemoryType> {
        Ok(MemoryType {
            memory64: ty.memory64,
            minimum: ty.initial,
            maximum: ty.maximum,
            shared: ty.shared,
        })
    }

    pub fn global_type(glob_type: &wasmparser::GlobalType) -> Result<GlobalType> {
        Ok(GlobalType {
            val_type: ty(&glob_type.content_type)?,
            mutable: glob_type.mutable,
        })
    }

    pub fn tag_type(tag_type: &wasmparser::TagType) -> Result<TagType> {
        Ok(TagType {
            kind: TagKind::Exception,
            func_type_idx: tag_type.func_type_idx,
        })
    }

    pub fn ty(ty: &wasmparser::ValType) -> Result<ValType> {
        match ty {
            wasmparser::ValType::I32 => Ok(ValType::I32),
            wasmparser::ValType::I64 => Ok(ValType::I64),
            wasmparser::ValType::F32 => Ok(ValType::F32),
            wasmparser::ValType::F64 => Ok(ValType::F64),
            wasmparser::ValType::V128 => Ok(ValType::V128),
            wasmparser::ValType::Ref(ty) => Ok(ValType::Ref(refty(ty)?)),
        }
    }

    pub fn refty(ty: &wasmparser::RefType) -> Result<RefType> {
        Ok(RefType {
            nullable: ty.nullable,
            heap_type: heapty(&ty.heap_type)?,
        })
    }

    pub fn heapty(ty: &wasmparser::HeapType) -> Result<HeapType> {
        match ty {
            wasmparser::HeapType::Func => Ok(HeapType::Func),
            wasmparser::HeapType::Extern => Ok(HeapType::Extern),
            wasmparser::HeapType::TypedFunc(i) => Ok(HeapType::TypedFunc((*i).into())),
        }
    }

    pub fn global(global: wasmparser::Global) -> Result<(GlobalType, Instruction)> {
        let ty = global_type(&global.ty)?;
        let init_expr: wasmparser::ConstExpr = global.init_expr;
        let instr: Instruction = const_expr_instr(&init_expr)?;
        Ok((ty, instr))
    }

    pub fn const_expr_instr<'a>(e: &wasmparser::ConstExpr<'a>) -> Result<Instruction<'a>> {
        let mut e = e.get_operators_reader();
        let operator = e.read()?;
        match e.read()? {
            Operator::End if e.eof() => {}
            _ => return Err(Error::msg("invalid init expression")),
        }
        op(&operator)
    }

    pub fn const_expr_element_function(
        e: &wasmparser::ConstExpr,
    ) -> Result<wasm_encoder::ConstExpr> {
        let instr = const_expr_instr(e)?;
        match instr {
            Instruction::RefNull(HeapType::Func)
            | Instruction::RefFunc(_)
            | Instruction::GlobalGet(_) => {}
            _ => return Err(Error::msg("no mutations applicable")),
        }

        Ok(constexpr_from_instr(instr))
    }

    pub fn const_expr(e: &wasmparser::ConstExpr) -> Result<wasm_encoder::ConstExpr> {
        let instr = const_expr_instr(e)?;

        Ok(constexpr_from_instr(instr))
    }

    pub fn element(element: wasmparser::Element) -> Result<ElementSegment> {
        let mode: ElementMode = match &element.kind {
            wasmparser::ElementKind::Active {
                table_index,
                offset_expr,
            } => {
                let offset_constexpr = const_expr(offset_expr)?;
                ElementMode::Active {
                    table: Some(*table_index),
                    offset: offset_constexpr,
                }
            }
            wasmparser::ElementKind::Passive => ElementMode::Passive,
            wasmparser::ElementKind::Declared => ElementMode::Declared,
        };
        let element_type = refty(&element.ty)?;
        let elements = match element.items {
            wasmparser::ElementItems::Functions(reader) => {
                let functions = reader.into_iter().collect::<Result<Vec<_>, _>>()?;
                Elements::Functions(functions)
            }
            wasmparser::ElementItems::Expressions(reader) => {
                let exprs = reader
                    .into_iter()
                    .map(|f| const_expr_element_function(&f?))
                    .collect::<Result<Vec<_>, _>>()?;
                Elements::Expressions(exprs)
            }
        };
        Ok(ElementSegment {
            mode,
            element_type,
            elements,
        })
    }

    #[allow(unused_variables)]
    pub fn op<'a>(op: &Operator) -> Result<Instruction<'a>> {
        use Instruction as I;

        macro_rules! translate {
            ($( @$proposal:ident $op:ident $({ $($arg:ident: $argty:ty),* })? => $visit:ident)*) => {
                Ok(match op {
                    $(
                        wasmparser::Operator::$op $({ $($arg),* })? => {
                            $(
                                $(let $arg = translate!(map $arg $arg);)*
                            )?
                            translate!(build $op $($($arg)*)?)
                        }
                    )*
                })
            };

            // This case is used to map, based on the name of the field, from the
            // wasmparser payload type to the wasm-encoder payload type through
            // `translator` as applicable.
            (map $arg:ident tag_index) => (*$arg);
            (map $arg:ident function_index) => (*$arg);
            (map $arg:ident table) => (*$arg);
            (map $arg:ident table_index) => (*$arg);
            (map $arg:ident table) => (*$arg);
            (map $arg:ident dst_table) => (*$arg);
            (map $arg:ident src_table) => (*$arg);
            (map $arg:ident type_index) => (*$arg);
            (map $arg:ident global_index) => (*$arg);
            (map $arg:ident mem) => (*$arg);
            (map $arg:ident src_mem) => (*$arg);
            (map $arg:ident dst_mem) => (*$arg);
            (map $arg:ident data_index) => (*$arg);
            (map $arg:ident elem_index) => (*$arg);
            (map $arg:ident blockty) => (block_type($arg)?);
            (map $arg:ident relative_depth) => (*$arg);
            (map $arg:ident targets) => ((
                $arg
                    .targets()
                    .collect::<Result<Vec<_>, wasmparser::BinaryReaderError>>()?
                    .into(),
                $arg.default(),
            ));
            (map $arg:ident table_byte) => (());
            (map $arg:ident mem_byte) => (());
            (map $arg:ident flags) => (());
            (map $arg:ident ty) => (ty($arg)?);
            (map $arg:ident hty) => (heapty($arg)?);
            (map $arg:ident memarg) => (memarg($arg)?);
            (map $arg:ident local_index) => (*$arg);
            (map $arg:ident value) => ($arg);
            (map $arg:ident lane) => (*$arg);
            (map $arg:ident lanes) => (*$arg);

            // This case takes the arguments of a wasmparser instruction and creates
            // a wasm-encoder instruction. There are a few special cases for where
            // the structure of a wasmparser instruction differs from that of
            // wasm-encoder.
            (build $op:ident) => (I::$op);
            (build BrTable $arg:ident) => (I::BrTable($arg.0, $arg.1));
            (build I32Const $arg:ident) => (I::I32Const(*$arg));
            (build I64Const $arg:ident) => (I::I64Const(*$arg));
            (build F32Const $arg:ident) => (I::F32Const(f32::from_bits($arg.bits())));
            (build F64Const $arg:ident) => (I::F64Const(f64::from_bits($arg.bits())));
            (build V128Const $arg:ident) => (I::V128Const($arg.i128()));
            (build $op:ident $arg:ident) => (I::$op($arg));
            (build CallIndirect $ty:ident $table:ident $_:ident) => (I::CallIndirect {
                ty: $ty,
                table: $table,
            });
            (build ReturnCallIndirect $ty:ident $table:ident) => (I::ReturnCallIndirect {
                ty: $ty,
                table: $table,
            });
            (build MemoryGrow $mem:ident $_:ident) => (I::MemoryGrow($mem));
            (build MemorySize $mem:ident $_:ident) => (I::MemorySize($mem));
            (build $op:ident $($arg:ident)*) => (I::$op { $($arg),* });
        }

        for_each_operator!(translate)
    }

    pub fn block_type(block_type: &wasmparser::BlockType) -> Result<BlockType> {
        match block_type {
            wasmparser::BlockType::Empty => Ok(BlockType::Empty),
            wasmparser::BlockType::Type(val_type) => Ok(BlockType::Result(ty(val_type)?)),
            wasmparser::BlockType::FuncType(f) => Ok(BlockType::FunctionType(*f)),
        }
    }

    pub fn memarg(memarg: &wasmparser::MemArg) -> Result<MemArg> {
        Ok(MemArg {
            offset: memarg.offset,
            align: memarg.align.into(),
            memory_index: memarg.memory,
        })
    }

    pub fn data(data: wasmparser::Data) -> Result<Data> {
        let mode: DataMode = match &data.kind {
            wasmparser::DataKind::Active {
                memory_index,
                offset_expr,
            } => {
                let offset_instr = const_expr_instr(offset_expr)?;
                DataMode::Active {
                    memory_index: *memory_index,
                    offset_instr,
                }
            }
            wasmparser::DataKind::Passive => DataMode::Passive,
        };
        let data = data.data.to_vec();
        Ok(Data { mode, data })
    }

    pub fn code(body: FunctionBody<'_>) -> Result<Function> {
        let locals = body
            .get_locals_reader()?
            .into_iter()
            .map(|local| {
                let (cnt, val_type) = local?;
                Ok((cnt, ty(&val_type)?))
            })
            .collect::<Result<Vec<_>>>()?;
        let mut func = Function::new(locals);

        let mut reader = body.get_operators_reader()?;
        reader.allow_memarg64(true);
        for operator in reader {
            let operator = operator?;
            func.instruction(&op(&operator)?);
        }
        Ok(func)
    }
}
