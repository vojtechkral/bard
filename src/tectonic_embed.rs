use std::iter;

use tectonic::config::PersistentConfig;
use tectonic::driver;
use tectonic::status::plain::PlainStatusBackend;
use tectonic::status::termcolor::TermcolorStatusBackend;
use tectonic::status::{ChatterLevel, StatusBackend};
use tectonic::unstable_opts::{UnstableArg, UnstableOptions};
use tectonic_bridge_core::{SecuritySettings, SecurityStance};

use crate::app::App;
use crate::prelude::*;

trait TectonicResultExt<T> {
    fn anyhow(self) -> Result<T>;
}

impl<T> TectonicResultExt<T> for Result<T, tectonic::Error> {
    fn anyhow(self) -> Result<T> {
        self.map_err(|err| anyhow!("{}", err))
    }
}

#[derive(clap::Parser)]
#[clap(
    about = "Embedded Tectonic interface, used internally by bard when rendering PDFs.
This interface is private and NOT recommended for general usage."
)]
pub struct Tectonic {
    #[arg(short, default_value_t = true, help = "Keep intermediate files")]
    keep: bool,
    #[arg(short, default_value_t = 0, help = "Max number of re-runs")]
    reruns: u32,
    #[arg(short, help = "Output directory path")]
    out_dir: Option<PathBuf>,

    input: PathBuf,
}

impl Tectonic {
    pub fn run(self, app: &App) -> Result<()> {
        let chatter = if app.verbosity() > 0 {
            ChatterLevel::Normal
        } else {
            ChatterLevel::Minimal
        };

        let mut status = if app.use_color() {
            Box::new(PlainStatusBackend::new(chatter)) as Box<dyn StatusBackend>
        } else {
            Box::new(TermcolorStatusBackend::new(chatter))
        };

        let config = PersistentConfig::open(false)
            .anyhow()
            .context("Failed to open default bundle")?;
        let bundle = config
            .default_bundle(false, &mut *status)
            .anyhow()
            .context("Failed to load the default resource bundle")?;
        let format_cache_path = config
            .format_cache_path()
            .anyhow()
            .context("Failed to set up the format cache")?;

        let file_name = self
            .input
            .file_name()
            .and_then(|os| os.to_str())
            .ok_or_else(|| anyhow!("Could not get filename of input path: {:?}", self.input))?;

        let security = SecuritySettings::new(SecurityStance::MaybeAllowInsecures); // so that extra search path option can be set
        let mut sb = driver::ProcessingSessionBuilder::new_with_security(security);
        sb.bundle(bundle)
            .primary_input_path(&self.input)
            .tex_input_name(file_name)
            .format_name("latex")
            .format_cache_path(format_cache_path)
            .keep_intermediates(self.keep)
            .keep_logs(self.keep)
            .reruns(self.reruns as _)
            .print_stdout(app.verbosity() >= 2)
            .output_format(driver::OutputFormat::Pdf);

        if let Some(out_dir) = self.out_dir.as_ref() {
            sb.output_dir(out_dir)
                // A workaround for https://github.com/tectonic-typesetting/tectonic/issues/981
                // see also TexConfig::render_args()
                .unstables(UnstableOptions::from_unstable_args(iter::once(
                    UnstableArg::SearchPath(out_dir.clone()),
                )));
        }

        let mut sess = sb
            .create(&mut *status)
            .anyhow()
            .context("Failed to initialize the LaTeX processing session")?;

        let res = sess.run(&mut *status);
        if let Err(tectonic::Error(tectonic::ErrorKind::EngineError(..), ..)) = &res {
            let output = sess.get_stdout_content();
            if !output.is_empty() {
                status.dump_error_logs(&output);
            }
        }

        res.anyhow().context("The LaTeX engine failed")
    }
}
