use async_trait::async_trait;
use loco_rs::prelude::*;
use loco_rs::task::Vars;

use crate::models::users;

pub struct UserReport;
#[cfg_attr(not(rust_analyzer), async_trait)]
impl Task for UserReport {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "user_report".to_string(),
            detail: "Output a user report".to_string(),
        }
    }

    // Variables through the CLI:
    //  `$ cargo loco task name:foobar count:2`
    // will appear as {"name": "foobar", "count": 2} in `vars`.
    async fn run(&self, app_context: &AppContext, vars: &Vars) -> Result<()> {
        let users = users::Entity::find().all(&app_context.db).await?;
        println!("args: {vars:?}");
        println!("!!! user_report: listing users !!!");
        println!("");
        for user in &users {
            println!("user: {}", user.email);
        }
        println!("Done: {} users", users.len());
        Ok(())
    }
}
