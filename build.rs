use std::env;
use std::fs;
use std::path::PathBuf;

fn push_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn write_res_entry(out: &mut Vec<u8>, res_type: u16, res_id: u16, data: &[u8]) {
    push_u32(out, data.len() as u32);
    push_u32(out, 32); // HeaderSize for numeric Type + numeric Name
    push_u16(out, 0xFFFF);
    push_u16(out, res_type);
    push_u16(out, 0xFFFF);
    push_u16(out, res_id);
    push_u32(out, 0); // DataVersion
    push_u16(out, 0x1030); // MOVEABLE | PURE | DISCARDABLE
    push_u16(out, 0x0409); // en-US
    push_u32(out, 0); // Version
    push_u32(out, 0); // Characteristics
    out.extend_from_slice(data);
    while out.len() % 4 != 0 {
        out.push(0);
    }
}

fn main() {
    println!("cargo:rerun-if-changed=assets/pinchzoom.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let ico = fs::read("assets/pinchzoom.ico").expect("failed to read assets/pinchzoom.ico");
    assert!(ico.len() >= 6, "invalid ico header");
    let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;

    let mut res = Vec::new();
    // Standard 32-byte empty leading resource header
    for &w in &[0u32, 32, 0x0000_FFFF, 0x0000_FFFF, 0, 0, 0, 0] {
        push_u32(&mut res, w);
    }

    let mut grp = Vec::with_capacity(6 + count * 14);
    grp.extend_from_slice(&ico[0..6]);

    for i in 0..count {
        let entry = 6 + i * 16;
        let bytes_in_res = u32::from_le_bytes([
            ico[entry + 8],
            ico[entry + 9],
            ico[entry + 10],
            ico[entry + 11],
        ]) as usize;
        let image_offset = u32::from_le_bytes([
            ico[entry + 12],
            ico[entry + 13],
            ico[entry + 14],
            ico[entry + 15],
        ]) as usize;
        let icon_id = (i + 1) as u16;

        // RT_ICON = 3
        write_res_entry(&mut res, 3, icon_id, &ico[image_offset..image_offset + bytes_in_res]);

        // GRPICONDIRENTRY: first 12 bytes of ICONDIRENTRY + 2-byte resource ID
        grp.extend_from_slice(&ico[entry..entry + 12]);
        push_u16(&mut grp, icon_id);
    }

    // RT_GROUP_ICON = 14, ID = 1 (default app icon in Explorer)
    write_res_entry(&mut res, 14, 1, &grp);

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let res_path = out_dir.join("pinchzoom.res");
    fs::write(&res_path, &res).expect("failed to write pinchzoom.res");

    println!("cargo:rustc-link-arg-bins={}", res_path.display());
}
