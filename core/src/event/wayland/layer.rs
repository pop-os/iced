use cctk::sctk::reexports::client::protocol::wl_output::WlOutput;

/// layer surface events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerEvent {
    /// layer surface Done
    Done,
    /// layer surface focused
    Focused,
    /// layer_surface unfocused
    Unfocused,
    /// layer surface is now shown (at least partly) on this output
    ///
    /// Lets surfaces placed by the compositor, e.g. on the active output,
    /// learn which output that is.
    OutputEnter {
        /// the output entered
        output: WlOutput,
        /// its name (e.g. `DP-1`), if the compositor announced one
        ///
        /// Included because output events sent before an application starts
        /// listening are not replayed, so it may not know the name otherwise.
        name: Option<String>,
    },
}
