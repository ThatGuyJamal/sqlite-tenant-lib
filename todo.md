# Todo list

- [x] Improve error handling with custom error types
- [ ] Documentation for public functions and structs
- [x] Configuration options for customizable behavior
- [x] Logging support for visibility into internal operations
- [x] Master Db uses SQL transactions for all writes
- [ ] Migration support for managing schema changes
- [ ] Mechanisms for tenant data isolation
- [ ] Comprehensive unit tests covering all edge cases
- [ ] Integration tests for real-world scenarios
- [ ] Example usage demonstrating various scenarios
- [ ] Engage with the Rust community for feedback and contributions


## Known Bugs

- [Example](./examples/user-management.rs) is having problems saving colum id. Need to fix this from being null on tenant inserts...