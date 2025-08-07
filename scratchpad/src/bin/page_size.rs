use scratchpad::println;

fn main() {
  println!(
    "the current page size is {}",
    aubystd::platform::active::rt::get_page_size()
  );
}
