use std::{env, fmt::Display, fs, io::stdin, process::exit};

use owo_colors::OwoColorize;

#[derive(Debug)]
struct BFInterpreter {
	memory: Vec<u8>,
	mem_ptr: usize,
	program: Vec<DebugCommand>,
	program_ptr: usize,
	output: Vec<u8>,
	input: Vec<u8>,
	input_ptr: usize,
	state: State,
	steps: usize,
	watchers: Vec<MemoryWatcher>,
}

#[derive(Debug)]
struct MemoryWatcher {
	index: usize,
	value: u8,
}

#[derive(Debug, Default, PartialEq)]
enum State {
	#[default]
	Running,
	TooFarLeft,
	EndOfProgram,
	StoppedOnMemoryValue,
	BreakPointHit,
}

#[derive(Debug)]
struct DebugCommand {
	command: Command,
	line_number: usize,
	column: usize,
}

#[derive(Debug, Clone, Copy)]
enum Command {
	Inc,
	Dec,
	Right,
	Left,
	Read,
	Write,
	BeginLoop(usize),
	EndLoop(usize),
	Break,
	End,
}

fn main() {
	let args: Vec<_> = env::args().collect();
	if args.len() <= 1 {
		println!("usage: brainfuck source_file input_file");
		exit(0);
	}
	let filename = &args[1];
	let source = fs::read_to_string(filename).unwrap_or_else(|err| {
		println!("Error reading file: {err}");
		exit(1);
	});
	let input_data = args
		.get(2)
		.map(|path| {
			fs::read(path).unwrap_or_else(|err| {
				println!("Error reading file: {err}");
				exit(1);
			})
		})
		.unwrap_or_default();

	let program = parse(&source);

	// dbg!(&code_dbg);
	let mut interpreter = BFInterpreter::new(program, input_data);
	loop {
		interpreter.show();
		let mut action = String::new();
		stdin().read_line(&mut action).unwrap();
		let action: Vec<_> = action.trim().split_ascii_whitespace().collect();
		match action.as_slice() {
			["step"] => interpreter.step_once(),
			["step", num] => _ = num.parse().map(|n| interpreter.step(n)),
			["watch"] => println!("usage: watch [memory index] [value]"),
			["watch", _] => println!("usage: watch [memory index] [value]"),
			["watch", index, value] => {
				if let (Ok(index), Ok(value)) = (index.parse(), value.parse()) {
					interpreter.add_watch(index, value)
				} else {
					println!(
						"{}",
						"index and value must be valid usize and u8 integers".red()
					);
				}
			}
			["run"] => interpreter.run(),
			["q" | "exit" | "quit"] => break,
			[] => interpreter.step_once(),
			_ => println!("{}", "unrecognised command".red()),
		}
	}
}

impl BFInterpreter {
	fn new(program: Vec<DebugCommand>, input: Vec<u8>) -> Self {
		Self {
			memory: vec![0],
			mem_ptr: 0,
			program,
			program_ptr: 0,
			output: Vec::new(),
			input,
			input_ptr: 0,
			state: State::Running,
			steps: 0,
			watchers: Vec::new(),
		}
	}

	fn show(&self) {
		for (index, c) in self.program.iter().enumerate() {
			if index == self.program_ptr {
				print!("{}", c.command.on_cyan());
			} else {
				print!("{}", c.command);
			}
		}
		println!();
		println!(
			"source: {}:{}",
			self.program[self.program_ptr].line_number, self.program[self.program_ptr].column
		);
		print!("mem: ");
		for (index, cell) in self.memory.iter().enumerate() {
			if index == self.mem_ptr {
				print!("{:3} ", cell.on_red());
			} else {
				print!("{:3} ", cell);
			}
		}
		println!();
		print!("ind: ");
		for i in 0..self.memory.len() {
			if i == self.mem_ptr {
				print!("{:3} ", i.on_red());
			} else {
				print!("{:3} ", i);
			}
		}
		println!();
		println!("{:?}. steps: {}", self.state, self.steps);
		println!("output: {}", String::from_utf8_lossy(&self.output));
		// println!("input: {}", String::from_utf8_lossy(&self.input));
	}

	fn add_watch(&mut self, index: usize, value: u8) {
		self.watchers.push(MemoryWatcher { index, value });
	}

	fn step_once(&mut self) {
		self.state = State::Running;
		self.step_internal();
	}

	fn step(&mut self, num: usize) {
		for _ in 0..num {
			self.step_internal();
			if self.state != State::Running {
				break;
			}
		}
	}

	fn run(&mut self) {
		while self.state == State::Running {
			self.step_internal();
		}
	}

	fn step_internal(&mut self) {
		if self.program_ptr + 1 == self.program.len() {
			self.state = State::EndOfProgram;
		}
		if self.state != State::Running {
			return;
		}
		let command = self.program[self.program_ptr].command;
		match command {
			Command::Inc => {
				self.memory[self.mem_ptr] = self.memory[self.mem_ptr].wrapping_add(1);
				self.update_watchers();
			}
			Command::Dec => {
				self.memory[self.mem_ptr] = self.memory[self.mem_ptr].wrapping_sub(1);
				self.update_watchers();
			}
			Command::Right => {
				self.mem_ptr += 1;
				if self.mem_ptr >= self.memory.len() {
					self.memory.push(0);
				}
			}
			Command::Left => {
				if self.mem_ptr == 0 {
					self.state = State::TooFarLeft;
				} else {
					self.mem_ptr -= 1;
				}
			}
			Command::Read => {
				if self.input_ptr < self.input.len() {
					self.memory[self.mem_ptr] = self.input[self.input_ptr];
					self.input_ptr += 1;
				} else {
					self.memory[self.mem_ptr] = 0;
				}
			}
			Command::Write => self.output.push(self.memory[self.mem_ptr]),
			Command::BeginLoop(end_of_loop) => {
				if self.memory[self.mem_ptr] == 0 {
					self.program_ptr = end_of_loop;
				}
			}
			Command::EndLoop(start_of_loop) => {
				if self.memory[self.mem_ptr] != 0 {
					self.program_ptr = start_of_loop;
				}
			}
			Command::Break => self.state = State::BreakPointHit,
			Command::End => (),
		}

		self.program_ptr += 1;
		self.steps += 1;
	}

	fn update_watchers(&mut self) {
		for watcher in &self.watchers {
			if watcher.index == self.mem_ptr && self.memory[watcher.index] == watcher.value {
				self.state = State::StoppedOnMemoryValue;
			}
		}
	}
}

fn parse(source_text: &str) -> Vec<DebugCommand> {
	let mut out: Vec<DebugCommand> = Vec::new();
	let mut loop_starts = Vec::new();
	for (line_number, line) in source_text
		.lines()
		.enumerate()
		.map(|(num, line)| (num + 1, line))
	{
		for (column, char) in line.chars().enumerate() {
			let cmd = match char {
				'+' => Command::Inc,
				'-' => Command::Dec,
				'>' => Command::Right,
				'<' => Command::Left,
				',' => Command::Read,
				'.' => Command::Write,
				'[' => {
					loop_starts.push(out.len());
					Command::BeginLoop(usize::MAX)
				}
				']' => {
					if loop_starts.is_empty() {
						println!("Parser error: no opening bracket for closing bracket at {line_number}:{column}");
						exit(1);
					}
					let last_loop_start = loop_starts.pop().unwrap();
					out[last_loop_start].command = Command::BeginLoop(out.len());

					Command::EndLoop(last_loop_start)
				}
				'!' => Command::Break,
				_ => continue,
			};
			out.push(DebugCommand {
				command: cmd,
				line_number,
				column,
			});
		}
	}
	out.push(DebugCommand {
		command: Command::End,
		line_number: 0,
		column: 0,
	});
	if let Some(loop_start_index) = loop_starts.pop() {
		let loop_start = &out[loop_start_index];
		println!(
			"Parser error: no matching closing bracket for open bracket at {}:{}",
			loop_start.line_number, loop_start.column
		);
		exit(1);
	}
	out
}

impl Display for Command {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Command::Inc => '+',
				Command::Dec => '-',
				Command::Right => '>',
				Command::Left => '<',
				Command::Read => ',',
				Command::Write => '.',
				Command::BeginLoop(_) => '[',
				Command::EndLoop(_) => ']',
				Command::Break => '!',
				Command::End => ' ',
			}
		)
	}
}
