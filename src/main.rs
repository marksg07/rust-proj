use my_project;
mod raqote_example;
mod chess;

fn main() -> my_project::Result<()> {
    println!("Hello, world!");
    raqote_example::main();
    //my_project::run(1500)?;
    Ok(())
}
