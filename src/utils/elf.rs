use byteorder::{ReadBytesExt, BE, LE};
use std::io;
use std::io::{Cursor, SeekFrom};
use tokio::io::{AsyncBufRead, AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt};

#[derive(Debug, Copy, Clone)]
pub struct ElfHeaderIdent {
    _actual: [u8; 16],
    _class: ElfClass,
    _byte_order: ElfByteOrder,
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct ElfHeader {
    _ident: ElfHeaderIdent,
    object_type: u16,
    pub(crate) machine: u16,
    version: u32,
    entry: u64,
    program_header_offset: u64,
    section_header_offset: u64,
    flags: u32,
    elf_header_size: u16,
    program_header_entry_size: u16,
    program_header_entries: u16,
    section_header_entry_size: u16,
    section_header_entries: u16,
    section_header_index_for_string_table: u16,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
enum ElfClass {
    Class32 = 1,
    Class64 = 2,
}

impl ElfClass {
    pub fn width(&self) -> usize {
        match self {
            ElfClass::Class32 => 4,
            ElfClass::Class64 => 8,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ElfByteOrder {
    Lsb = 1,
    Msb = 2,
}

impl ElfHeader {
    const ELFCLASS32: u8 = 1;
    const ELFCLASS64: u8 = 2;

    const ELFDATA2LSB: u8 = 1;
    const ELFDATA2MSB: u8 = 2;

    const _ET_NONE: u16 = 0;
    const _ET_REL: u16 = 1;
    const ET_EXEC: u16 = 2;
    const ET_DYN: u16 = 3;
    const _ET_CORE: u16 = 4;

    const _PT_INTERP: u32 = 3;

    pub fn is_shared_object(&self) -> bool {
        self.object_type == ElfHeader::ET_DYN
    }

    pub fn is_executable(&self) -> bool {
        self.object_type == ElfHeader::ET_EXEC
    }

    pub async fn parse<R: AsyncBufRead + Unpin>(input: &mut R) -> anyhow::Result<Option<Self>> {
        let mut header = [0u8; 56];
        let amt_read = input.read(&mut header).await?;
        if amt_read < 24 || &header[..4] != b"\x7FELF" {
            return Ok(None);
        }

        let class_flag = header[4];
        let class = match class_flag {
            Self::ELFCLASS32 => ElfClass::Class32,
            Self::ELFCLASS64 => ElfClass::Class64,
            _ => return Ok(None),
        };

        let byte_order = match header[5] {
            Self::ELFDATA2LSB => ElfByteOrder::Lsb,
            Self::ELFDATA2MSB => ElfByteOrder::Msb,
            _ => return Ok(None),
        };

        let ident = ElfHeaderIdent {
            _actual: header[..16].try_into().unwrap(),
            _class: class,
            _byte_order: byte_order,
        };

        let rest_header_size = 40 + (class.width() * 3);
        if amt_read < rest_header_size {
            return Ok(None);
        }

        let mut cur = Cursor::new(&header[16..]);

        let header = match byte_order {
            ElfByteOrder::Lsb => ElfHeader {
                _ident: ident,
                object_type: ReadBytesExt::read_u16::<LE>(&mut cur)?,
                machine: ReadBytesExt::read_u16::<LE>(&mut cur)?,
                version: ReadBytesExt::read_u32::<LE>(&mut cur)?,
                entry: match class {
                    ElfClass::Class32 => ReadBytesExt::read_u32::<LE>(&mut cur)? as u64,
                    ElfClass::Class64 => ReadBytesExt::read_u64::<LE>(&mut cur)?,
                },
                program_header_offset: match class {
                    ElfClass::Class32 => ReadBytesExt::read_u32::<LE>(&mut cur)? as u64,
                    ElfClass::Class64 => ReadBytesExt::read_u64::<LE>(&mut cur)?,
                },
                section_header_offset: match class {
                    ElfClass::Class32 => ReadBytesExt::read_u32::<LE>(&mut cur)? as u64,
                    ElfClass::Class64 => ReadBytesExt::read_u64::<LE>(&mut cur)?,
                },
                flags: ReadBytesExt::read_u32::<LE>(&mut cur)?,
                elf_header_size: ReadBytesExt::read_u16::<LE>(&mut cur)?,
                program_header_entry_size: ReadBytesExt::read_u16::<LE>(&mut cur)?,
                program_header_entries: ReadBytesExt::read_u16::<LE>(&mut cur)?,
                section_header_entry_size: ReadBytesExt::read_u16::<LE>(&mut cur)?,
                section_header_entries: ReadBytesExt::read_u16::<LE>(&mut cur)?,
                section_header_index_for_string_table: ReadBytesExt::read_u16::<LE>(&mut cur)?,
            },
            ElfByteOrder::Msb => ElfHeader {
                _ident: ident,
                object_type: ReadBytesExt::read_u16::<BE>(&mut cur)?,
                machine: ReadBytesExt::read_u16::<BE>(&mut cur)?,
                version: ReadBytesExt::read_u32::<BE>(&mut cur)?,
                entry: match class {
                    ElfClass::Class32 => ReadBytesExt::read_u32::<BE>(&mut cur)? as u64,
                    ElfClass::Class64 => ReadBytesExt::read_u64::<BE>(&mut cur)?,
                },
                program_header_offset: match class {
                    ElfClass::Class32 => ReadBytesExt::read_u32::<BE>(&mut cur)? as u64,
                    ElfClass::Class64 => ReadBytesExt::read_u64::<BE>(&mut cur)?,
                },
                section_header_offset: match class {
                    ElfClass::Class32 => ReadBytesExt::read_u32::<BE>(&mut cur)? as u64,
                    ElfClass::Class64 => ReadBytesExt::read_u64::<BE>(&mut cur)?,
                },
                flags: ReadBytesExt::read_u32::<BE>(&mut cur)?,
                elf_header_size: ReadBytesExt::read_u16::<BE>(&mut cur)?,
                program_header_entry_size: ReadBytesExt::read_u16::<BE>(&mut cur)?,
                program_header_entries: ReadBytesExt::read_u16::<BE>(&mut cur)?,
                section_header_entry_size: ReadBytesExt::read_u16::<BE>(&mut cur)?,
                section_header_entries: ReadBytesExt::read_u16::<BE>(&mut cur)?,
                section_header_index_for_string_table: ReadBytesExt::read_u16::<BE>(&mut cur)?,
            },
        };

        Ok(Some(header))
    }

    pub async fn _quick_find_interpreter<R: AsyncRead + AsyncSeek + Unpin>(
        &self,
        input: &mut R,
    ) -> io::Result<bool> {
        input
            .seek(SeekFrom::Start(self.program_header_offset))
            .await?;

        let mut buffer = vec![0u8; self.program_header_entry_size as usize];
        for _ in 0..self.program_header_entries {
            input.read_exact(&mut buffer).await?;
            let entry_type = match self._ident._byte_order {
                ElfByteOrder::Lsb => u32::from_le_bytes((&buffer[..4]).try_into().unwrap()),
                ElfByteOrder::Msb => u32::from_be_bytes((&buffer[..4]).try_into().unwrap()),
            };

            if entry_type == Self::_PT_INTERP {
                return Ok(true);
            }
        }

        Ok(false)
    }
}
