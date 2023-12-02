use std::{env, fs, io::stdin, process::exit};

#[derive(Debug)]
struct StateMachine {
	memory: Vec<u8>,
	mem_ptr: usize,
	program: Vec<DebugCommand>,
	program_ptr: usize,
	output: Vec<u8>,
	input: Vec<u8>,
	input_ptr: usize,
	state: State,
}

#[derive(Debug, Default, PartialEq)]
enum State {
	#[default]
	Running,
	TooFarLeft,
	EndOfProgram,
}

#[derive(Debug)]
struct DebugCommand {
	command: Command,
	line_number: usize,
	column: usize,
}

// #[derive(Debug)]
// struct FastCommand {
// 	command: Command,
// 	count: u8,
// }

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

	for c in &program {
		print!("{}", c.command.char());
	}
	println!();
	// dbg!(&code_dbg);
	let mut state_machine = StateMachine::new(program, input_data);
	loop {
		println!("{:?}", state_machine.memory);
		println!("{:?}", state_machine.state);
		println!("output: {}", String::from_utf8_lossy(&state_machine.output));
		let mut action = String::new();
		stdin().read_line(&mut action).unwrap();
		action = action.trim().to_owned();
		if action.starts_with("step ") {
			if let Ok(num) = action[5..].trim().parse() {
				state_machine.step(num);
			}
		}
		match action.as_str() {
			"step" => state_machine.step_once(),
			"run" => state_machine.run(),
			"exit" | "quit" => break,
			_ => (),
		}
	}
}

impl StateMachine {
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
		}
	}

	fn step(&mut self, num: usize) {
		for _ in 0..num {
			self.step_once();
			if self.state != State::Running {
				break;
			}
		}
	}

	fn run(&mut self) {
		while self.state == State::Running {
			self.step_once();
		}
	}

	fn step_once(&mut self) {
		if self.program_ptr >= self.program.len() {
			self.state = State::EndOfProgram;
		}
		if self.state != State::Running {
			return;
		}
		let command = self.program[self.program_ptr].command;
		match command {
			Command::Inc => self.memory[self.mem_ptr] = self.memory[self.mem_ptr].wrapping_add(1),
			Command::Dec => self.memory[self.mem_ptr] = self.memory[self.mem_ptr].wrapping_sub(1),
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
		}

		self.program_ptr += 1;
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
				_ => continue,
			};
			out.push(DebugCommand {
				command: cmd,
				line_number,
				column,
			});
		}
	}
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

impl Command {
	fn char(&self) -> char {
		match self {
			Command::Inc => '+',
			Command::Dec => '-',
			Command::Right => '>',
			Command::Left => '<',
			Command::Read => ',',
			Command::Write => '.',
			Command::BeginLoop(_) => '[',
			Command::EndLoop(_) => ']',
		}
	}
}
