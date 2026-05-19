use crate::db;
use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use sqlx::PgPool;
use std::io::{self, Write};

pub async fn add_user(pool: &PgPool) -> Result<()> {
    println!("=== Atom - 新增使用者 ===");

    let username = prompt("Username: ")?;
    if username.is_empty() {
        anyhow::bail!("Username 不可為空");
    }

    // rpassword hides input characters (no echo to terminal)
    let password = rpassword::prompt_password("Password: ")?;
    if password.len() < 8 {
        anyhow::bail!("密碼至少需要 8 個字元");
    }

    let confirm = rpassword::prompt_password("Confirm password: ")?;
    if password != confirm {
        anyhow::bail!("兩次輸入的密碼不一致");
    }

    let hash = hash_password(&password)?;

    db::crud::create_user(pool, &username, &hash).await?;

    println!("✅ 使用者 '{username}' 建立成功");
    Ok(())
}

fn prompt(label: &str) -> Result<String> {
    print!("{label}");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Password hash failed: {e}"))?
        .to_string();
    Ok(hash)
}
