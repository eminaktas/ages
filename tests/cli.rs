use assert_cmd::Command;
use predicates::prelude::*;

fn ages(dir: &std::path::Path) -> Command {
    let mut c = Command::cargo_bin("ages").unwrap();
    c.arg("--data-dir").arg(dir);
    c.env_remove("AGES_HOME");
    c
}

#[test]
fn add_list_edit_remove_flow() {
    let d = tempfile::tempdir().unwrap();
    ages(d.path())
        .args([
            "add",
            "anne",
            "--name",
            "Ayşe",
            "--surname",
            "Aktaş",
            "--birth",
            "14.03.1965",
            "--time",
            "04:30",
            "--tz",
            "Europe/Istanbul",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("anne"));
    ages(d.path())
        .args(["add", "kardes", "--name", "Can", "--birth", "2000-08-01"])
        .assert()
        .success();
    ages(d.path()).arg("list").assert().success().stdout(
        predicate::str::contains("Ayşe Aktaş")
            .and(predicate::str::contains("Pisces"))
            .and(predicate::str::contains("Leo")),
    );
    let out = ages(d.path()).args(["list", "--json"]).output().unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v[0]["alias"], "anne");
    assert!(v[0]["age"]["years"].as_u64().unwrap() >= 61);
    assert_eq!(v[0]["zodiac"], "pisces");
    assert_eq!(v[0]["tz"], "Europe/Istanbul");
    assert!(v[1]["age"].get("hours").is_none());
    assert!(v[1]["next_birthday_in_days"].is_number());
    ages(d.path())
        .args(["edit", "anne", "--rename", "mom", "--surname", "Yılmaz"])
        .assert()
        .success();
    ages(d.path())
        .arg("list")
        .assert()
        .stdout(predicate::str::contains("mom").and(predicate::str::contains("Yılmaz")));
    ages(d.path())
        .args(["remove", "mom", "--yes"])
        .assert()
        .success();
    ages(d.path())
        .arg("list")
        .assert()
        .stdout(predicate::str::contains("mom").not());
}

#[test]
fn duplicate_alias_exit_1() {
    let d = tempfile::tempdir().unwrap();
    ages(d.path())
        .args(["add", "a", "--name", "A", "--birth", "2000-01-01"])
        .assert()
        .success();
    ages(d.path())
        .args(["add", "a", "--name", "A", "--birth", "2000-01-01"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn bad_date_exit_1() {
    let d = tempfile::tempdir().unwrap();
    ages(d.path())
        .args(["add", "a", "--name", "A", "--birth", "31.02.2000"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("not a date"));
}

#[test]
fn bad_avatar_does_not_create_entry() {
    let d = tempfile::tempdir().unwrap();
    let t = d.path().join("x.txt");
    std::fs::write(&t, "no").unwrap();
    ages(d.path())
        .args([
            "add",
            "a",
            "--name",
            "A",
            "--birth",
            "2000-01-01",
            "--avatar",
        ])
        .arg(&t)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("as an image"));
    ages(d.path())
        .args(["list", "--json"])
        .assert()
        .stdout("[]\n");
}

#[test]
fn avatar_imported_and_removed() {
    let d = tempfile::tempdir().unwrap();
    let img = d.path().join("in.png");
    image::RgbImage::from_fn(50, 80, |_, y| image::Rgb([0, y as u8, 0]))
        .save(&img)
        .unwrap();
    ages(d.path())
        .args([
            "add",
            "a",
            "--name",
            "A",
            "--birth",
            "2000-01-01",
            "--avatar",
        ])
        .arg(&img)
        .assert()
        .success();
    let stored = d.path().join("avatars/a.png");
    assert!(stored.exists());
    let out = ages(d.path()).args(["list", "--json"]).output().unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v[0]["avatar"].as_str().unwrap().ends_with("avatars/a.png"));
    ages(d.path())
        .args(["edit", "a", "--no-avatar"])
        .assert()
        .success();
    assert!(!stored.exists());
    ages(d.path())
        .args(["remove", "a", "-y"])
        .assert()
        .success();
}

#[test]
fn edit_rename_with_bad_date_leaves_avatar_in_place() {
    let d = tempfile::tempdir().unwrap();
    let img = d.path().join("in.png");
    image::RgbImage::from_fn(20, 20, |_, _| image::Rgb([1, 2, 3]))
        .save(&img)
        .unwrap();
    ages(d.path())
        .args([
            "add",
            "anne",
            "--name",
            "A",
            "--birth",
            "2000-01-01",
            "--avatar",
        ])
        .arg(&img)
        .assert()
        .success();
    ages(d.path())
        .args(["edit", "anne", "--rename", "mom", "--birth", "31.02.2000"])
        .assert()
        .code(1);
    assert!(d.path().join("avatars/anne.png").exists());
    assert!(!d.path().join("avatars/mom.png").exists());
    ages(d.path())
        .arg("list")
        .assert()
        .stdout(predicate::str::contains("anne"));
}

#[test]
fn remove_without_yes_on_non_tty_fails() {
    let d = tempfile::tempdir().unwrap();
    ages(d.path())
        .args(["add", "a", "--name", "A", "--birth", "2000-01-01"])
        .assert()
        .success();
    ages(d.path())
        .args(["remove", "a"])
        .write_stdin("")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("--yes"));
    ages(d.path())
        .arg("list")
        .assert()
        .stdout(predicate::str::contains("A"));
}

#[test]
fn turkish_headers() {
    let d = tempfile::tempdir().unwrap();
    ages(d.path())
        .args(["add", "a", "--name", "A", "--birth", "2000-03-01"])
        .assert()
        .success();
    ages(d.path())
        .args(["--lang", "tr", "list"])
        .assert()
        .stdout(predicate::str::contains("Balık").and(predicate::str::contains("takma ad")));
    ages(d.path())
        .args(["--lang", "tr", "--help"])
        .assert()
        .stdout(predicate::str::contains("Kişi ekle"));
}

#[test]
fn settings_lang_is_used_when_no_flag() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("people.toml"), "[settings]\nlang = \"tr\"\n").unwrap();
    ages(d.path())
        .arg("list")
        .assert()
        .stdout(predicate::str::contains("Henüz kimse yok"));
}

#[test]
fn bare_ages_in_pipe_prints_table() {
    let d = tempfile::tempdir().unwrap();
    ages(d.path())
        .args(["add", "a", "--name", "Ali", "--birth", "2000-01-01"])
        .assert()
        .success();
    ages(d.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Ali"));
}

#[test]
fn empty_store_hint() {
    let d = tempfile::tempdir().unwrap();
    ages(d.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("ages add"));
}

#[test]
fn usage_error_exit_2() {
    let d = tempfile::tempdir().unwrap();
    ages(d.path()).args(["add", "a"]).assert().code(2);
}

#[test]
fn ages_home_env_is_respected() {
    let d = tempfile::tempdir().unwrap();
    let mut c = Command::cargo_bin("ages").unwrap();
    c.env("AGES_HOME", d.path());
    c.args(["add", "a", "--name", "A", "--birth", "2000-01-01"])
        .assert()
        .success();
    assert!(d.path().join("people.toml").exists());
}

#[test]
fn unicode_alias_with_spaces_round_trips_with_avatar() {
    let d = tempfile::tempdir().unwrap();
    let img = d.path().join("in.png");
    image::RgbImage::from_fn(20, 20, |_, _| image::Rgb([1, 2, 3]))
        .save(&img)
        .unwrap();
    ages(d.path())
        .args([
            "add",
            "Küçük Kardeş",
            "--name",
            "Can",
            "--birth",
            "2000-01-01",
            "--avatar",
        ])
        .arg(&img)
        .assert()
        .success()
        .stdout(predicate::str::contains("Küçük Kardeş"));
    assert!(d.path().join("avatars/Küçük Kardeş.png").exists());
    ages(d.path())
        .args(["edit", "Küçük Kardeş", "--rename", " Eşim "])
        .assert()
        .success();
    assert!(d.path().join("avatars/Eşim.png").exists());
    let out = ages(d.path()).args(["list", "--json"]).output().unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v[0]["alias"], "Eşim");
    ages(d.path())
        .args(["add", "a/b", "--name", "A", "--birth", "2000-01-01"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("invalid"));
}

#[test]
fn pixel_avatar_is_stored_pixelated() {
    let d = tempfile::tempdir().unwrap();
    let img = d.path().join("in.png");
    image::RgbImage::from_fn(200, 200, |x, y| image::Rgb([x as u8, y as u8, 90]))
        .save(&img)
        .unwrap();
    // --pixel without --avatar is a usage error.
    ages(d.path())
        .args([
            "add",
            "a",
            "--name",
            "A",
            "--birth",
            "2000-01-01",
            "--pixel",
        ])
        .assert()
        .code(2);
    ages(d.path())
        .args([
            "add",
            "a",
            "--name",
            "A",
            "--birth",
            "2000-01-01",
            "--pixel",
            "16",
            "--avatar",
        ])
        .arg(&img)
        .assert()
        .success();
    let out = ages(d.path()).args(["list", "--json"]).output().unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v[0]["pixel"], true);
    assert!(
        std::fs::read_to_string(d.path().join("people.toml"))
            .unwrap()
            .contains("pixel = true")
    );
    let stored = image::open(d.path().join("avatars/a.png"))
        .unwrap()
        .to_rgba8();
    let colors: std::collections::HashSet<[u8; 4]> = stored.pixels().map(|p| p.0).collect();
    assert!(colors.len() <= 32, "{}", colors.len());
    assert_eq!(stored.get_pixel(0, 0), stored.get_pixel(15, 15));
    // Re-importing without --pixel goes back to a smooth photo.
    ages(d.path())
        .args(["edit", "a", "--avatar"])
        .arg(&img)
        .assert()
        .success();
    let out = ages(d.path()).args(["list", "--json"]).output().unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v[0]["pixel"], false);
    let stored = image::open(d.path().join("avatars/a.png"))
        .unwrap()
        .to_rgba8();
    let colors: std::collections::HashSet<[u8; 4]> = stored.pixels().map(|p| p.0).collect();
    assert!(colors.len() > 32);
    ages(d.path())
        .args(["edit", "a", "--no-avatar"])
        .assert()
        .success();
    let out = ages(d.path()).args(["list", "--json"]).output().unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v[0]["pixel"], false);
    assert!(v[0]["avatar"].is_null());
}
