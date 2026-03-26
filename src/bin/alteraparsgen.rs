use alteraparser::meta::codegen::CodeGeneratorBuilder;
use clap::Parser;

#[derive(clap::Parser, Debug, Clone)]
#[command(author, version)]
#[command(
    help_template = "{name} - {about} [version: {version}, author: {author}]\n\n{usage-heading} {usage}\n\n{all-args}"
)]
#[command(about="Code generator for an alteraparser grammar", long_about = None)]
pub struct Options {
    #[arg(help = "Grammar input file")]
    pub grammar_input_file: String,

    #[arg(short = 'o', long = "output", help = "Stop after parsing")]
    pub grammar_output_file: Option<String>,

    #[arg(
        long = "indent-size",
        help = "Indent size for generated code",
        default_value_t = 2
    )]
    pub indent_size: usize,

    #[arg(
        long = "grammar-fn",
        help = "Name of the grammar constructor function",
        default_value = "make_grammar"
    )]
    pub grammar_fn: String,
}

fn main() {
    let options = Options::parse();

    let grammar_content = std::fs::read_to_string(&options.grammar_input_file)
        .expect("Failed to read grammar input file");

    let top_comment = format!(
        "Generated from grammar file: {}",
        options.grammar_input_file
    );

    let code_generator = CodeGeneratorBuilder::new()
        .indent_size(options.indent_size)
        .top_comment(&top_comment)
        .function_name(&options.grammar_fn)
        .build();

    let grammar_code = code_generator
        .generate_code(&grammar_content)
        .expect("Failed to generate grammar code");

    if let Some(output_file) = options.grammar_output_file {
        std::fs::write(&output_file, grammar_code).expect("Failed to write grammar output file");
    } else {
        println!("{}", grammar_code);
    }
}
