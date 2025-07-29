import { build_error, Range } from "./universal_error_system.js";
import {
  FUNS,
  jsSession,
  ARG_GIVEN,
  ARG_PASSED,
  ARG_LOCATED,
} from "./interpreter.js";

const OPERATORS = ["+", "-", "*", "/"];
const COMPILED_EXPRESSION = 12;
const NEGATIVE_SIGN = 10;
const TRANSLATE_OPERATOR = 9;
const SCALE_OPERATOR = 8;
const PARENTHESIS = 5;
const OPERATOR = 4;
const INTEGER = 3;
const WHITESPACE = 0;

const SCOPE_GLOBAL = 1;

const PASSED = 1;
const FAILED = 0;

const CONFIG_IGNORE = 0;
const CONFIG_USE = 1;

const OPERATOR_ANCHORS = {
  island: {
    8: {
      "*": "integer_multiply",
      "/": "integer_divide",
    },
    9: {
      "+": "integer_add",
      "-": "integer_subtract",
    },
  },
};

class Slice {
  constructor(type, slice, config) {
    this.type = type;
    this.slice = slice;
    this.config = config;
  }
}

function parse_expression(args) {
  let parsed_state = PASSED;
  let slices = [];
  let expected_types = [];
  let errors = [];

  let scope = SCOPE_GLOBAL;

  for (let i = 0; i < args.length; i++) {
    const slice = args[i];
    let temporially_slice = NaN;
    let next_types = [];

    if (!isNaN(Number(slice)) && slice !== " ") {
      temporially_slice = new Slice(INTEGER, Number(slice), CONFIG_USE);

      next_types.push(TRANSLATE_OPERATOR);
      next_types.push(SCALE_OPERATOR);
      next_types.push(WHITESPACE);
    } else if (OPERATORS.includes(slice)) {
      if (expected_types.includes(NEGATIVE_SIGN) && slice == "-") {
        temporially_slice = new Slice(NEGATIVE_SIGN, slice, CONFIG_USE);
      } else {
        if (slice == "+" || slice == "-") {
          temporially_slice = new Slice(TRANSLATE_OPERATOR, slice, CONFIG_USE);
        }
        if (slice == "*" || slice == "/") {
          temporially_slice = new Slice(SCALE_OPERATOR, slice, CONFIG_USE);
        }
        next_types.push(NEGATIVE_SIGN);
      }

      next_types.push(INTEGER);
      next_types.push(WHITESPACE);
    } else if (slice == " ") {
      temporially_slice = new Slice(WHITESPACE, slice, CONFIG_IGNORE);
      //console.log(expected_types)
    }

    if (
      expected_types.length < 1 ||
      expected_types.includes(temporially_slice.type)
    ) {
      if (temporially_slice.config == CONFIG_USE) {
        slices.push(temporially_slice);
        expected_types = next_types;
      }
    } else {
      parsed_state = FAILED;
      errors.push(
        build_error(
          "split string",
          "0",
          "logic",
          2,
          "main.azl",
          32,
          new Range(12, 14),
        ),
      );
    }
  }

  if (parsed_state == PASSED) {
    return [parsed_state, slices];
  } else {
    return [parsed_state, errors];
  }
}

function compile_expression(slices, session) {
  let cells = [];
  let current_cres_pos = 0;

  for (var i = 0; i < slices.length; i++) {
    let slice = slices[i];
    if (slice.type == NEGATIVE_SIGN) {
      if (slices[i + 1].type == INTEGER) {
        cells.push(
          FUNS["arithmetic"]["integer_multiply"].bind(
            this,
            session,
            [-1, Number(slices[i + 1].slice), NaN],
            [ARG_GIVEN, ARG_GIVEN, ARG_GIVEN],
            true,
          ),
        );
        slices[i] = new Slice(
          COMPILED_EXPRESSION,
          current_cres_pos,
          CONFIG_IGNORE,
        );
        slices.splice(i + 1, 1);
        i--;
        //current_cres_pos++;
      } else {
        // throw error: expected int found x
      }
    }
  }

  compile_island_operator(
    SCALE_OPERATOR,
    cells,
    slices,
    current_cres_pos,
    session,
  );
  compile_island_operator(
    TRANSLATE_OPERATOR,
    cells,
    slices,
    current_cres_pos,
    session,
  );

  return cells;
}

/*  Just some error message example
    -----------[error::logic::002]----------------
    ☐ Expected a "int", found an "opt"
    ☐ Found on line 1, character 5, of main.azl
        1 | 3 + - 2
                ^--- cause of error
*/

function compile_island_operator(
  opt_set,
  cells,
  slices,
  current_cres_pos,
  session,
) {
  for (var k = 0; k < slices.length; k++) {
    let slice = slices[k];
    if (slice.type == opt_set) {
      let func_name = NaN;
      let scope = OPERATOR_ANCHORS["island"][opt_set];
      for (var j = 0; j < Object.keys(scope).length; j++) {
        if (slice.slice == Object.keys(scope)[j]) {
          func_name = scope[Object.keys(scope)[j]];
        }
      }

      let val1 = { locs: NaN, slice: NaN };
      if (slices[k - 1].type == COMPILED_EXPRESSION) {
        val1.locs = ARG_PASSED;
        val1.slice = slices[k - 1].slice;
      } else if (slices[k - 1].type == INTEGER) {
        val1.locs = ARG_GIVEN;
        val1.slice = slices[k - 1].slice;
      }

      let val2 = { locs: NaN, slice: NaN };
      if (slices[k + 1].type == COMPILED_EXPRESSION) {
        val2.locs = ARG_PASSED;
        val2.slice = slices[k + 1].slice;
      } else if (slices[k + 1].type == INTEGER) {
        val2.locs = ARG_GIVEN;
        val2.slice = slices[k + 1].slice;
      }

      cells.push(
        FUNS["arithmetic"][func_name].bind(
          this,
          session,
          [val1.slice, val2.slice, NaN],
          [val1.locs, val2.locs, ARG_GIVEN],
          true,
        ),
      );
      slices[k + 1] = new Slice(
        COMPILED_EXPRESSION,
        current_cres_pos,
        CONFIG_IGNORE,
      );
      slices.splice(k - 1, 2);
      k -= 2;
    }
  }
}

function main() {
  // Just a sample expression for testing :)
  const exp = [
    "12",
    " ",
    "+",
    " ",
    " ",
    "2",
    "-",
    "2",
    "-",
    "5",
    "+",
    "33",
    "*",
    "2",
    "/",
    "3",
  ];

  // First we tokenize the expression to make compilation easier. The function returns a list where
  // the first element is 0 if parsing failed, and 1 if it succeeded. The second element is a list
  // of the the tokens.
  let result = parse_expression(exp);
  console.log(result);

  if (result[0] == FAILED) {
    // If the expression failed to tokenize properly, we print the errors and return.
    for (var i = 0; i < result[1].length; i++) {
      console.log(result[1][i]);
      return;
    }
  } else {
    // If the expression tokenized properly, we can now compile it into multiple js functions calls
    // which are bound to the session object.
    var session = new jsSession("", "main.azl");
    var rtcells = compile_expression(result[1], session);
    console.log(rtcells);

    // Finally, we execute the compiled function calls.
    for (var r = 0; r < rtcells.length; r++) {
      rtcells[r]();
    }
    console.log(session);
  }
}

main();
