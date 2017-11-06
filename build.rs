#![feature(ascii_ctype)]

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

fn or_name<'a>(caption: &'a String, name: &'a String) -> &'a String {
    if caption.len() == 0 {
        name
    } else {
        caption
    }
}

/// "ADC Noise Reduction (If Available)" -> "ADC_Noise_Reduction_If_Available"
fn caption_to_ident(caption: &String, group_name: &String) -> String {
    let ident: String = caption
        .chars()
        .filter_map(|c| if c.is_ascii_whitespace() {
            Some('_')
        } else if c.is_ascii_alphanumeric() {
            Some(c)
        } else {
            None
        })
        .collect();

    format!("{}_{}", group_name, ident)
}

fn genmcu(outdir: &PathBuf, mcu: &avr_mcu::Mcu) -> std::io::Result<()> {
    let mut mcu_def = File::create(outdir.join("mcudef.rs"))?;

    writeln!(mcu_def, "// MCU defs for {}", mcu.device.name)?;

    // Interrupt handler definitions.  These assume that we're linking
    // with the avr libc startup and that it will take care of putting
    // these specially named functions into the interrupt vector table.
    writeln!(mcu_def, "/// Helper for declaring IRQ handlers.")?;
    writeln!(mcu_def, "/// Possible handlers are:")?;
    for interrupt in mcu.device.interrupts.iter() {
        writeln!(mcu_def, "/// irq_handler!({}, my_fn);", interrupt.name)?;
    }
    writeln!(mcu_def, "#[macro_export]")?;
    writeln!(mcu_def, "macro_rules! irq_handler {{")?;
    for interrupt in mcu.device.interrupts.iter() {
        writeln!(mcu_def, "    ({}, $path:path) => {{", interrupt.name)?;
        writeln!(mcu_def, "    /// {}", interrupt.caption)?;
        writeln!(mcu_def, "    #[no_mangle]")?;
        writeln!(
            mcu_def,
            "    pub unsafe extern \"avr-interrupt\" fn __vector_{}() {{",
            interrupt.index
        )?;
        writeln!(mcu_def, "        let f: fn() = $path;")?;
        writeln!(mcu_def, "        f();")?;
        writeln!(mcu_def, "    }}")?;
        writeln!(mcu_def, " }};")?;
    }
    writeln!(mcu_def, " }}")?;

    let mut defined_signal_flags = HashSet::new();

    let mut instance_by_name = HashMap::new();
    for p in mcu.device.peripherals.iter() {
        for inst in p.instances.iter() {
            // Emit a cfg flag that we can test in the handwritten
            // code to tell whether a given peripheral instance
            // is available in the handwritten code.
            println!("cargo:rustc-cfg=AVR_{}", inst.name);
            instance_by_name.insert(&inst.name, inst);
        }
    }

    // Candidate locations for simavr registers
    let mut simavr_console_reg = None;
    let mut simavr_command_reg = None;

    // Emit registers
    for module in mcu.modules.iter() {
        writeln!(mcu_def, "/// Registers for {}", mcu.device.name)?;

        let mut value_group_by_name = HashMap::new();
        for vg in module.value_groups.iter() {
            value_group_by_name.insert(&vg.name, vg);
        }

        for group in module.register_groups.iter() {
            writeln!(mcu_def, "")?;
            writeln!(mcu_def, "/// Register group: {}", group.name)?;
            writeln!(mcu_def, "/// {}", group.caption)?;

            let struct_name =
                RenameRule::PascalCase.apply_to_field(group.name.to_ascii_lowercase());

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
                let mut done_value_name: HashSet<String> = HashSet::new();

                for field in reg.bitfields.iter() {
                    if let Some(ref value_group) = field.values {
                        // References a handy precompute mask.  The labels for these are
                        // a bit hit and miss, so we generate two versions; one using the
                        // name and one using the caption text so that we double the chances
                        // of getting a meaningful result.
                        let vg = value_group_by_name.get(value_group).expect(&format!(
                            "broken value group linkage from field {:?}",
                            field
                        ));

                        for value in vg.values.iter() {
                            let name = caption_to_ident(&value.caption, &vg.name);
                            if done_value_name.contains(&name) {
                                continue;
                            }
                            done_value_name.insert(name.clone());

                            // We need to modify the value from the group to match the mask
                            // on the current field.  We do this by shifting off bits from the
                            // mask until we no longer have zero bits on the LSB end.  The
                            // number of bits shifted off is the number of bits we need to
                            // shift the value group value in the opposite direction.
                            let mut vg_value = value.value;
                            let mut mask = field.mask;
                            while mask & 1 == 0 {
                                mask = mask >> 1;
                                vg_value = vg_value << 1;
                            }

                            bitfields.push((name, vg_value, or_name(&value.caption, &value.name)));
                        }
                    } else {
                        bitfields.push((
                            field.name.clone(),
                            field.mask,
                            or_name(&field.caption, &field.name),
                        ));
                    }
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
                    if let Some(inst) = instance_by_name.get(&group.name) {
                        for sig in inst.signals.iter() {
                            if let Some(bitno) = sig.index {
                                bitfields.push((sig.pad.clone(), 1 << bitno, &sig.pad));

                                defined_signal_flags.insert(flags_name.clone());
                            }
                        }
                    }
                }

                if bitfields.len() == 0 {
                    continue;
                }

                have_reg_type.insert(&reg.name, flags_name.clone());

                writeln!(mcu_def, "regflags! {{")?;
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

            writeln!(mcu_def, "#[repr(C, packed)]")?;
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

                        // Can we stick the simavr special regs in this hole?
                        let mut reg_space = hole_size;
                        let mut reg_addr = prior.offset + prior.size;
                        if simavr_console_reg.is_none() {
                            simavr_console_reg = Some(reg_addr);
                            reg_space -= 1;
                            reg_addr += 1;
                        }

                        if simavr_command_reg.is_none() && reg_space > 0 {
                            simavr_command_reg = Some(reg_addr);
                        }
                    }
                }

                let reg_name =
                    RenameRule::SnakeCase.apply_to_field(register.name.to_ascii_lowercase());
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
    }

    writeln!(mcu_def, "\n\n\n")?;
    writeln!(mcu_def, "#[cfg(feature=\"simavr\")]")?;
    writeln!(mcu_def, "pub mod simavr_regs {{")?;
    writeln!(mcu_def, "use super::*;")?;
    writeln!(mcu_def, "use simavr;")?;
    writeln!(mcu_def, "use volatile_register::RW;")?;

    if let Some(simavr_console_reg) = simavr_console_reg {
        writeln!(mcu_def, "/// Simavr console")?;
        writeln!(
            mcu_def,
            "pub const SIMAVR_CONSOLE: Peripheral<RW<u8>> =\
             unsafe {{ Peripheral::new({}) }};",
            simavr_console_reg
        )?;

        writeln!(mcu_def, "#[no_mangle]")?;
        writeln!(mcu_def, "#[link_section = \".mmcu\"]")?;
        writeln!(
            mcu_def,
            "pub static SIMAVR_CONSOLE_REG: simavr::McuAddr =\
             simavr::McuAddr {{ tag: simavr::Tag::TAG_SIMAVR_CONSOLE, \
             len: 1, what: {} as *const u8 }};",
            simavr_console_reg
        )?;
    }

    if let Some(simavr_command_reg) = simavr_command_reg {
        writeln!(mcu_def, "/// Simavr command")?;
        writeln!(
            mcu_def,
            "pub const SIMAVR_COMMAND: Peripheral<RW<u8>> =\
             unsafe {{ Peripheral::new({}) }};",
            simavr_command_reg
        )?;

        writeln!(mcu_def, "#[no_mangle]")?;
        writeln!(mcu_def, "#[link_section = \".mmcu\"]")?;
        writeln!(
            mcu_def,
            "pub static SIMAVR_COMMAND_REG: simavr::McuAddr =\
             simavr::McuAddr {{ tag: simavr::Tag::TAG_SIMAVR_COMMAND,\
             len: 1, what: {} as *const u8 }};",
            simavr_command_reg
        )?;
    }

    writeln!(mcu_def, "}}")?;

    Ok(())
}
