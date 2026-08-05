// Compile FIRST so the directive applies to all subsequent files.
// Without it Icarus defaults to 1s precision and ns timers misbehave.
`timescale 1ns/1ns
