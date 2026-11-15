//! CLI utility for importing localization/assets and verifying `.sav` files.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

use sr_persist::GameV12;
use sr_tools::{import_decompile_localization, AssetManifest, ToolsResult};

fn print_usage() {
    eprintln!(
        "sr_tools usage:\n  \
         sr_tools import-loc <decompile_dir> [out_localization_dir]\n  \
         sr_tools scan-assets [local_assets_dir]\n  \
         sr_tools verify-save <path_to.sav>"
    );
}

fn cmd_import_loc(args: &[String]) -> ToolsResult<()> {
    let src = args
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("decompile"));
    let out = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("local_assets/localization"));
    let results = import_decompile_localization(&src, &out)?;
    for line in results {
        println!("[ok] {line}");
    }
    Ok(())
}

fn cmd_scan_assets(args: &[String]) -> ToolsResult<()> {
    let root = args
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("local_assets"));
    let manifest = AssetManifest::scan(&root)?;
    let out_path = root.join("manifest.json");
    manifest.save_json(&out_path)?;
    println!(
        "[ok] scanned {}: {} textures, {} audio, {} models, {} localizations -> {}",
        root.display(),
        manifest.textures.len(),
        manifest.audio.len(),
        manifest.models.len(),
        manifest.localization.len(),
        out_path.display()
    );
    Ok(())
}

fn cmd_verify_save(args: &[String]) -> ToolsResult<()> {
    let Some(path_str) = args.first() else {
        print_usage();
        process::exit(2);
    };
    let path = PathBuf::from(path_str);
    let bytes = fs::read(&path)?;
    let v12 = GameV12::from_bytes(&bytes)?;
    let model = v12.into_game_model()?;
    let reencoded = GameV12::from_game_model(
        &model,
        &v12.game_name,
        &v12.display_name,
        v12.summary.save_number,
        v12.summary.save_timestamp_ticks,
    )
    .to_bytes()?;
    let reloaded = GameV12::from_bytes(&reencoded)?.into_game_model()?;

    println!("=== Save Verified: {} ===", path.display());
    println!("Game Name   : {}", v12.game_name);
    println!("Display Name: {}", v12.display_name);
    println!("App Version : {}", v12.summary.version);
    println!("Mode        : {:?}", model.game_mode);
    println!("Currency    : {}", model.player.currency);
    println!("World Time  : {:.2}s", model.world.world_time);
    println!("Pedia Count : {}", model.pedia.unlocked.len());
    println!("Land Plots  : {}", model.land_plots.len());
    println!("Gadget Sites: {}", model.gadget_sites.len());
    println!("Gordos      : {}", model.gordos.len());
    println!("Pods        : {}", model.pods.len());
    println!("Doors       : {}", model.doors.len());
    println!("Actors      : {} (round-trip: {})", model.actors.len(), reloaded.actors.len());
    println!(
        "Bytes       : {} in -> {} re-encoded",
        bytes.len(),
        reencoded.len()
    );
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some((cmd, rest)) = args.split_first() else {
        print_usage();
        process::exit(2);
    };

    let res = match cmd.as_str() {
        "import-loc" => cmd_import_loc(rest),
        "scan-assets" => cmd_scan_assets(rest),
        "verify-save" => cmd_verify_save(rest),
        _ => {
            print_usage();
            process::exit(2);
        }
    };

    if let Err(err) = res {
        eprintln!("[error] {err}");
        process::exit(1);
    }
}
