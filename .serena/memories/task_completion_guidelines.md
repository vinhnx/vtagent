# Task Completion Guidelines for vtagent

## Pre-Task Preparation

### Environment Setup
1. **Verify Environment**: Ensure Rust 1.75+ is installed
   ```bash
   rustc --version
   cargo --version
   ```

2. **Check Dependencies**: Verify all required tools are available
   ```bash
   ./scripts/setup.sh
   ```

3. **Set API Keys**: Ensure Gemini API key is configured
   ```bash
   export GEMINI_API_KEY=your_key_here
   ```

## Code Development Workflow

### 1. Planning Phase
- **Understand Requirements**: Read issue/PR description carefully
- **Explore Codebase**: Use `grep_search` or `semantic_search` to understand existing patterns
- **Identify Impact**: Determine which modules/components will be affected
- **Plan Changes**: Outline the implementation approach

### 2. Implementation Phase
- **Follow Code Style**: Adhere to established conventions
- **Write Tests First**: Implement tests before or alongside code
- **Error Handling**: Use appropriate error types and handling patterns
- **Documentation**: Add comprehensive documentation for new code

### 3. Quality Assurance Phase
- **Run Tests**: Execute relevant test suites
  ```bash
  cargo test
  cargo test --test integration_tests
  ```

- **Code Quality Checks**: Run all quality checks
  ```bash
  ./scripts/check.sh
  ```

- **Performance Testing**: Run benchmarks if performance-critical
  ```bash
  cargo bench
  ```

### 4. Documentation Phase
- **Update Documentation**: Update relevant docs in `docs/` directory
- **Code Comments**: Ensure all public APIs have proper documentation
- **Changelog**: Update `CHANGELOG.md` if applicable

## Specific Task Types

### Adding New Tools
1. **Define Tool**: Add tool definition in `src/tools.rs`
2. **Register Tool**: Add to tool registry in appropriate location
3. **Add Tests**: Create comprehensive tests in `tests/`
4. **Update Documentation**: Document tool usage and parameters
5. **Integration Test**: Add integration test for end-to-end functionality

### Modifying Existing Functionality
1. **Understand Current Implementation**: Read existing code thoroughly
2. **Preserve Compatibility**: Ensure changes don't break existing functionality
3. **Update Tests**: Modify existing tests and add new ones
4. **Migration Path**: Consider backward compatibility if needed

### Performance Improvements
1. **Benchmark Current Performance**: Establish baseline metrics
2. **Profile Code**: Identify performance bottlenecks
3. **Implement Optimizations**: Apply targeted improvements
4. **Verify Improvements**: Confirm performance gains with benchmarks

### Bug Fixes
1. **Reproduce Issue**: Create test case that reproduces the bug
2. **Identify Root Cause**: Debug and locate the source of the problem
3. **Implement Fix**: Apply minimal, targeted fix
4. **Add Regression Test**: Ensure bug cannot reoccur

## Testing Strategy

### Unit Tests
- **Coverage**: Aim for comprehensive unit test coverage
- **Isolation**: Test components in isolation
- **Edge Cases**: Test error conditions and edge cases
- **Mocking**: Use appropriate mocking for external dependencies

### Integration Tests
- **End-to-End**: Test complete workflows
- **Real Dependencies**: Use real dependencies where possible
- **Data Validation**: Test with realistic data sets
- **Error Scenarios**: Test failure modes and recovery

### Performance Tests
- **Benchmarks**: Use `cargo bench` for performance-critical code
- **Memory Usage**: Monitor memory consumption
- **Scalability**: Test with varying load sizes
- **Regression Prevention**: Track performance over time

## Code Review Preparation

### Self-Review Checklist
- [ ] Code follows established style guidelines
- [ ] All tests pass (`cargo test`)
- [ ] Code quality checks pass (`./scripts/check.sh`)
- [ ] Documentation is updated
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Code is properly formatted (`cargo fmt`)
- [ ] Error handling is appropriate
- [ ] Performance impact is considered
- [ ] Security implications are addressed

### Documentation Updates
- [ ] README.md updated if needed
- [ ] API documentation updated
- [ ] Code comments added for complex logic
- [ ] Changelog updated
- [ ] Migration guide if breaking changes

## Commit and Release Process

### Commit Guidelines
- **Atomic Commits**: Each commit should be a single, complete change
- **Clear Messages**: Write descriptive commit messages
- **Reference Issues**: Include issue/PR references when applicable
- **Test with Commit**: Ensure all tests pass before committing

### Release Preparation
- **Version Bump**: Update version in `Cargo.toml`
- **Changelog**: Update `CHANGELOG.md` with release notes
- **Documentation**: Ensure all documentation is current
- **Final Testing**: Run complete test suite
- **Build Verification**: Build in release mode successfully

## Common Pitfalls to Avoid

### Code Quality Issues
- **Ignoring Clippy**: Always address clippy warnings
- **Poor Error Messages**: Provide clear, actionable error messages
- **Missing Documentation**: Document all public APIs
- **Inconsistent Style**: Follow established code conventions

### Testing Issues
- **Missing Edge Cases**: Test error conditions and edge cases
- **Flaky Tests**: Ensure tests are deterministic and reliable
- **Slow Tests**: Keep unit tests fast, use integration tests for slower operations
- **Test Dependencies**: Don't create tests that depend on external services

### Performance Issues
- **Unnecessary Allocations**: Minimize heap allocations in hot paths
- **Blocking Operations**: Avoid blocking operations in async code
- **Memory Leaks**: Ensure proper resource cleanup
- **Scalability**: Consider performance impact at scale

### Security Issues
- **Input Validation**: Always validate user inputs
- **Path Traversal**: Prevent directory traversal attacks
- **Sensitive Data**: Never log or expose sensitive information
- **API Security**: Use secure communication protocols

## Getting Help

### Resources
- **Documentation**: Check `docs/` directory for detailed guides
- **Examples**: Review `examples/` for implementation patterns
- **Tests**: Look at existing tests for usage patterns
- **Issues**: Check GitHub issues for similar problems

### Communication
- **Clear Descriptions**: Provide clear problem descriptions
- **Code Examples**: Include relevant code snippets
- **Expected Behavior**: Describe desired vs. actual behavior
- **Environment Info**: Include relevant environment details