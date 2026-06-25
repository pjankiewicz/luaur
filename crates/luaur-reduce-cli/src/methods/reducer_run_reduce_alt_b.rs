use crate::enums::test_result::TestResult;
use crate::records::reducer::Reducer;
use luaur_ast::records::parser::Parser;
use luaur_ast::type_aliases::cst_node_map::CstNodeMap;
use std::process;

impl Reducer {
    pub fn run_string_string_string_view_string_view(
        &mut self,
        script_name: String,
        command: String,
        source: &str,
        search_text: &str,
    ) {
        self.script_name = script_name;

        println!("Script: {}", self.script_name);

        self.command = command;
        self.search_text = search_text.to_string();

        // `name_table` stored a `*mut Allocator` pointing at the local allocator in
        // `Reducer::new`; returning the Reducer by value relocated `self.allocator`,
        // leaving that pointer dangling. Re-point it at the current allocator before
        // parsing (the same pattern the test Fixture uses) — otherwise interning
        // identifiers dereferenced freed stack memory and segfaulted.
        self.name_table.rebind_allocator(&mut self.allocator);

        self.parse_result = Parser::parse(
            source,
            source.len(),
            &mut self.name_table,
            &mut self.allocator,
            self.parse_options.clone(),
        );
        if !self.parse_result.errors.is_empty() {
            println!("Parse errors");
            process::exit(1);
        }

        self.root = self.parse_result.root;
        self.cst_node_map = core::mem::replace(
            &mut self.parse_result.cst_node_map,
            CstNodeMap::new(core::ptr::null_mut()),
        );

        let initial_result = self.run();
        if initial_result == TestResult::NoBug {
            println!("Could not find failure string in the unmodified script!  Check your commandline arguments");
            process::exit(2);
        }

        self.walk(self.root);

        self.write_temp_script(true);

        println!("Done!  Check {}", self.script_name);
    }
}
