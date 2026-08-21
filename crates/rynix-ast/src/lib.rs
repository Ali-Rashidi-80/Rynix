//! Arena-allocated AST for Rynix.
//!
//! Implemented in Phase 2. Nodes live in a bump arena (`AstArena`); no
//! `Box`/`Rc`/`Drop` types are permitted inside nodes (ADR-0004).
