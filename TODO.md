### TODO

- `fix`: actually the GitHub Copilot LLMs are not working
- `feat`: it seems that the TDD trio will continue TDDing forever 😅, we should find a way of getting a completion point, or set a maximum amount of iterations
- `feat`: add a doc section with config examples?
- `feat`: is there a proper prompt for the implementor to avoid it being too greedy in implementing the feature beyond the scope of the currently failing test?
    - can we "penalize" the implementor for example if the next test passes immediately?
- `feat`: can we improve the way the refactorer does refactoring?
    - it seems to be very lazy in creating new files / modules
