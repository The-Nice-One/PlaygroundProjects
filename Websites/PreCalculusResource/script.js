// Background grid effect
document.addEventListener("DOMContentLoaded", () => {
  const gridContainer = document.getElementById("grid-container");
  const squares = [];

  const numberOfSquaresToCreate = 400;
  const fadeIntervalMs = 70;
  const stayFadedInMs = 1800;

  function createGrid() {
    gridContainer.innerHTML = "";
    squares.length = 0;

    for (let i = 0; i < numberOfSquaresToCreate; i++) {
      const square = document.createElement("div");
      square.classList.add("grid-square");
      gridContainer.appendChild(square);
      squares.push(square);
    }
  }

  function randomlyFadeSquare() {
    if (squares.length === 0) return;

    const randomIndex = Math.floor(Math.random() * squares.length);
    const randomSquare = squares[randomIndex];

    if (!randomSquare.classList.contains("fade-in")) {
      randomSquare.classList.add("fade-in");
      setTimeout(() => {
        randomSquare.classList.remove("fade-in");
      }, stayFadedInMs);
    }
  }

  createGrid();
  setInterval(randomlyFadeSquare, fadeIntervalMs);
});

// Navigation menu functionality
var menu_opened = false;

function toggle_menu() {
  if (menu_opened) {
    let button = document.getElementById("menu-arrow");
    button.textContent = "▼ Quick Links";
    menu_opened = false;
    document.getElementById("menu-links").classList.add("menu-links-closed");
  } else {
    let button = document.getElementById("menu-arrow");
    button.textContent = "▲ Quick Links";
    menu_opened = true;
    document.getElementById("menu-links").classList.remove("menu-links-closed");
  }
}

// Quiz functionality
let currentHint = 0;
let currentQuestion = 0;
let questions = [
  ["", "", "", "", "", "c"],
  [
    "Find the quotient of the following expression: \\( \\frac{10x^2+29x+10}{5x+2} \\)",
    "\\( 2x+5 \\)",
    "\\( 2x-5 \\)",
    "\\( 2x \\)",
    "\\( -2x \\)",
    "a",
  ],
  [
    "Find the quotient of the following expression: \\( \\frac{3x^3+23x^2+15x+7}{x+7} \\)",
    "\\( \\frac{1}{x+7} \\)",
    "\\( 3x^2+2x+\\frac{1}{x+7} \\)",
    "\\(  3x^2+2x \\)",
    "\\( 3x^2+2x+1 \\)",
    "d",
  ],
];

function checkAnswer() {
  document.querySelector(".feedback.correct").style.display = "none";
  document.querySelector(".feedback.incorrect").style.display = "none";

  const selectedOption = document.querySelector('input[name="q1"]:checked');

  if (selectedOption) {
    if (selectedOption.value === questions[currentQuestion][5]) {
      document.querySelector(".feedback.correct").style.display =
        "inline-block";
      if (currentQuestion >= 2) {
        document.getElementById("next-question-btn").style.display = "none";
      }
    } else {
      document.querySelector(".feedback.incorrect").style.display = "block";
    }
  }
}

function nextQuestion() {
  currentQuestion++;
  document.querySelector(".feedback.correct").style.display = "none";
  document.querySelector(".feedback.incorrect").style.display = "none";

  const radioButtons = document.querySelectorAll(
    `input[type="radio"][name="q1"]`,
  );
  radioButtons.forEach((radio) => {
    radio.checked = false;
  });

  document.getElementById("question-title").innerHTML =
    questions[currentQuestion][0];
  document.getElementById("q1a-label").innerHTML =
    questions[currentQuestion][1];
  document.getElementById("q1b-label").innerHTML =
    questions[currentQuestion][2];
  document.getElementById("q1c-label").innerHTML =
    questions[currentQuestion][3];
  document.getElementById("q1d-label").innerHTML =
    questions[currentQuestion][4];

  MathJax.typesetPromise(document.getElementsByClassName("quiz-question"));
}
