use std::{path::Path, sync::Arc};

use crossbeam_channel::{Receiver, Sender};
use pdb::TypeIndex;
use resym_core::{
    backend::{Backend, BackendCommand},
    frontend::{FrontendCommand, FrontendController},
    ResymCoreError,
};

use crate::error::Result;

/// Frontend implementation for the application
/// This struct enables the backend to communicate with us (the frontend)
pub struct ResymFrontendController {
    pub rx_ui: Receiver<FrontendCommand>,
    tx_ui: Sender<FrontendCommand>,
}

impl FrontendController for ResymFrontendController {
    /// Used by the backend to send us commands and trigger a UI update
    fn send_command(&self, command: FrontendCommand) -> resym_core::Result<()> {
        self.tx_ui
            .send(command)
            .map_err(|err| ResymCoreError::CrossbeamError(err.to_string()))
    }
}

impl ResymFrontendController {
    pub fn new(tx_ui: Sender<FrontendCommand>, rx_ui: Receiver<FrontendCommand>) -> Self {
        Self { rx_ui, tx_ui }
    }
}

const PDB_SLOT: usize = 0;

/// Struct that represents our CLI application.
/// It contains the whole application's context at all time.
pub struct WinDiffApp {
    frontend_controller: Arc<ResymFrontendController>,
    backend: Backend,
}

impl WinDiffApp {
    pub fn new() -> Result<Self> {
        // Initialize backend
        let (tx_ui, rx_ui) = crossbeam_channel::unbounded::<FrontendCommand>();
        let frontend_controller = Arc::new(ResymFrontendController::new(tx_ui, rx_ui));
        let backend = Backend::new(frontend_controller.clone())?;

        Ok(Self {
            frontend_controller,
            backend,
        })
    }

    pub fn extract_types_from_pdb(&self, pdb_path: &Path) -> Result<Vec<(String, String)>> {
        log::trace!("Extracting types from {:?}", pdb_path);

        // Load PDB
        self.load_pdb(pdb_path)?;

        // Retrieve a list of all types present in the PDB
        let type_list = self.list_types()?;

        // Reconstruct all the types
        let mut reconstructed_types = Vec::with_capacity(type_list.len());
        for (type_identifier, type_id) in type_list {
            if let Ok(recontructed_type) = self.reconstruct_type(type_id) {
                reconstructed_types.push((type_identifier, recontructed_type));
            }
        }

        self.unload_pdb()?;

        Ok(reconstructed_types)
    }

    fn load_pdb(&self, pdb_path: &Path) -> Result<()> {
        // Request the backend to load the PDB
        self.backend.send_command(BackendCommand::LoadPDBFromPath(
            PDB_SLOT,
            pdb_path.to_path_buf(),
        ))?;

        // Wait for the backend to finish loading the PDB
        if let FrontendCommand::LoadPDBResult(result) = self.frontend_controller.rx_ui.recv()? {
            if let Err(err) = result {
                return Err(crate::error::WinDiffError::ResymBackendError(format!(
                    "Failed to load PDB file: {}",
                    err
                )));
            }
        } else {
            return Err(crate::error::WinDiffError::ResymBackendError(
                "Invalid response received from the backend?".to_string(),
            ));
        }

        Ok(())
    }

    fn unload_pdb(&self) -> Result<()> {
        Ok(self
            .backend
            .send_command(BackendCommand::UnloadPDB(PDB_SLOT))?)
    }

    fn list_types(&self) -> Result<Vec<(String, TypeIndex)>> {
        // Queue a request for the backend to return the list of types that
        // match the given filter
        self.backend.send_command(BackendCommand::ListTypes(
            PDB_SLOT,
            String::default(),
            false,
            false,
            false,
        ))?;

        // Wait for the backend to finish filtering types
        if let FrontendCommand::ListTypesResult(type_list) =
            self.frontend_controller.rx_ui.recv()?
        {
            Ok(type_list)
        } else {
            Err(crate::error::WinDiffError::ResymBackendError(
                "Invalid response received from the backend?".to_string(),
            ))
        }
    }

    fn reconstruct_type(&self, type_id: TypeIndex) -> Result<String> {
        // Queue a request for the backend to reconstruct the type
        self.backend
            .send_command(BackendCommand::ReconstructTypeByIndex(
                PDB_SLOT,
                type_id,
                resym_core::pdb_types::PrimitiveReconstructionFlavor::Microsoft,
                false,
                false,
                false,
                false,
            ))?;

        // Wait for the backend to finish reconstructing the type
        if let FrontendCommand::ReconstructTypeResult(result) =
            self.frontend_controller.rx_ui.recv()?
        {
            let (reconstructed_type, _) = result?;

            Ok(reconstructed_type)
        } else {
            Err(crate::error::WinDiffError::ResymBackendError(
                "Invalid response received from the backend?".to_string(),
            ))
        }
    }
}
