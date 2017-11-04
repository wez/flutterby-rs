extern crate avr_mcu;
extern crate ident_case;

use std::env;
use std::path::PathBuf;
use std::fs::File;
use std::io::Write;
use std::ascii::AsciiExt;
use std::collections::{HashMap, HashSet};

use ident_case::RenameRule;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let outdir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mcu = avr_mcu::current::mcu().expect("must be building for an AVR target");

    genmcu(&outdir, &mcu).expect("failed to generate mcu data");
}

fn instance_for_register_group<'a>(
    mcu: &'a avr_mcu::Mcu,
    group_name: &String,
) -> Option<&'a avr_mcu::Instance> {
    for p in mcu.device.peripherals.iter() {
        for inst in p.instances.iter() {
            if inst.name == *group_name {
                return Some(inst);
            }
        }
    }

    None
}

fn or_name<'a>(caption: &'a String, name: &'a String) -> &'a String {
    if caption.len() == 0 {
        name
    } else {
        caption
    }
}

fn genmcu(outdir: &PathBuf, mcu: &avr_mcu::Mcu) -> std::io::Result<()> {
    let mut mcu_def = File::create(outdir.join("mcudef.rs"))?;

    writeln!(mcu_def, "// MCU defs for {}", mcu.device.name)?;

    let mut defined_signal_flags = HashSet::new();

    // Emit registers


    writeln!(mcu_def, "/// Registers for {}", mcu.device.name)?;
    for group in mcu.register_groups() {
        writeln!(mcu_def, "")?;
        writeln!(mcu_def, "/// Register group: {}", group.name)?;
        writeln!(mcu_def, "/// {}", group.caption)?;

        let struct_name = RenameRule::PascalCase.apply_to_field(group.name.to_ascii_lowercase());

        let mut sorted_regs = group.registers.clone();
        sorted_regs.sort_by(|a, b| a.offset.cmp(&b.offset));

        let mut have_reg_type = HashMap::new();

        // First pass through to define some bitfields
        for reg in sorted_regs.iter() {
            let flag_reg_name =
                RenameRule::PascalCase.apply_to_field(reg.name.to_ascii_lowercase());
            let mut flags_name = format!("{}{}Flags", struct_name, flag_reg_name);
            let register_type = if reg.size == 1 { "u8" } else { "u16" };

            let mut bitfields = Vec::new();

            for field in reg.bitfields.iter() {
                bitfields.push((
                    &field.name,
                    field.mask,
                    or_name(&field.caption, &field.name),
                ));
            }
            if bitfields.len() == 0 {
                // Let's see if we can find matching peripheral data
                flags_name = format!(
                    "{}SignalFlags",
                    RenameRule::PascalCase.apply_to_field(group.name.to_ascii_lowercase())
                );
                if defined_signal_flags.contains(&flags_name) {
                    // we already defined this struct
                    have_reg_type.insert(&reg.name, flags_name.clone());
                    continue;
                }
                if let Some(inst) = instance_for_register_group(mcu, &group.name) {
                    for sig in inst.signals.iter() {
                        if let Some(bitno) = sig.index {
                            bitfields.push((&sig.pad, 1 << bitno, &sig.pad));

                            defined_signal_flags.insert(flags_name.clone());
                        }
                    }
                }
            }

            if bitfields.len() == 0 {
                continue;
            }

            have_reg_type.insert(&reg.name, flags_name.clone());

            writeln!(mcu_def, "bitflags! {{")?;
            writeln!(
                mcu_def,
                "    pub struct {}: {} {{",
                flags_name,
                register_type
            )?;
            for (name, mask, caption) in bitfields {
                writeln!(mcu_def, "        /// {}", caption)?;
                writeln!(
                    mcu_def,
                    "        const {} = {};",
                    RenameRule::ScreamingSnakeCase.apply_to_field(&name),
                    mask
                )?;
            }
            writeln!(mcu_def, "    }}")?;
            writeln!(mcu_def, "}}")?;
        }

        writeln!(mcu_def, "#[repr(C)]")?;
        writeln!(mcu_def, "pub struct {} {{", struct_name)?;
        let mut num_holes = 0;
        let base_addr = sorted_regs[0].offset;
        for idx in 0..sorted_regs.len() {
            // do we need a hole?
            let register = &sorted_regs[idx];

            if idx > 0 {
                let prior = &sorted_regs[idx - 1];

                let hole_size = register.offset - (prior.offset + prior.size);
                if hole_size > 0 {
                    writeln!(mcu_def, "    reserved{}: [u8; {}],", num_holes, hole_size)?;
                    num_holes += 1;
                }
            }

            let reg_name = RenameRule::SnakeCase.apply_to_field(register.name.to_ascii_lowercase());
            let register_type = match have_reg_type.get(&register.name) {
                Some(name) => name.to_owned(),
                _ => if register.size == 1 {
                    "u8".to_owned()
                } else {
                    "u16".to_owned()
                },
            };

            writeln!(mcu_def, "    /// {}", register.name)?;

            let reg_wrapper = match register.rw {
                /* hmm, the source of this data doesn't appear to be what I want,
                 * so let's just map all of these to RW for now.
                avr_mcu::ReadWrite::ReadAndWrite => "volatile_register::RW",
                avr_mcu::ReadWrite::ReadOnly => "volatile_register::RO",
                avr_mcu::ReadWrite::WriteOnly => "volatile_register::WO",
                */
                _ => "volatile_register::RW",
            };

            writeln!(
                mcu_def,
                "    pub {}: {}<{}>,",
                reg_name,
                reg_wrapper,
                register_type
            )?;
        }
        writeln!(mcu_def, "}}")?; // end of struct def
        writeln!(mcu_def, "")?;

        writeln!(mcu_def, "/// {}", group.name)?;
        writeln!(mcu_def, "/// {}", group.caption)?;
        writeln!(
            mcu_def,
            "pub const {}: Peripheral<{}> = unsafe {{ Peripheral::new({}) }};",
            group.name,
            struct_name,
            base_addr
        )?;
    }

    Ok(())
}
