use anyhow::Result;
use ashlang::AshlangInnerProdArg;
use ashlang::Compiler;
use lettuce::*;
use zkpo::ZKArg;

static SRC: &'static str = "
(input_len, mat_height, mat_width)

let mat = read(mat_height * mat_width)
let vec = read(mat_width)

let out[mat_height]
static i = 0
loop mat_height {
    static j = 0
    let v = 0
    loop mat_width {
        v = v + vec[j] * mat[i * mat_width + j]
        j = j + 1
    }
    out[i] = v
    i = i + 1
}

#write_output(out)
";

fn main() -> Result<()> {
    let mut compiler = Compiler::<MilliScalarMont>::default();
    compiler.include(&"./ashlang/stdlib".into())?;
    let program = compiler.combine_entrypoint_src(SRC)?;
    println!("{}", program.src);

    let rng = &mut rand::rng();

    let mat = Matrix::<MilliScalarMont>::random(5, 10, rng);
    let vec = (0..10)
        .map(|_| MilliScalarMont::sample_uniform(rng))
        .collect::<Vector<_>>();

    let input = mat
        .iter()
        .flat_map(|v| v.iter())
        .cloned()
        .chain(vec.into_iter())
        .collect::<Vector<MilliScalarMont>>();
    println!(
        "{}",
        program.ar1cs_src(input.len(), &vec![mat.height(), mat.width()])?
    );
    let arg = AshlangInnerProdArg::new(program, input, &vec![mat.height(), mat.width()])?;
    arg.verify()?;

    Ok(())
}
