//! Background collector started by the LaunchAgent: records per-app network usage into the database.

fn main() -> Result<(), netmeter_lib::error::AppError> {
    netmeter_lib::collector::run()
}
