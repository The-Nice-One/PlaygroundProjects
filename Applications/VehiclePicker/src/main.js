// ComposeLibrary to create HTML elements programatically, written by me. Heavily
// inspired by IFM´s Sem library discussed at https://forum.code.org/t/an-applab-library-i-created/37006.
var Compose = new ComposeLibrary.init();
var $ = Compose.$;

var veichles = [
  { name: "Walking",    distance: 5,         icon: "icon://fa-male" },
  { name: "Bus",        distance: 8,         icon: "icon://fa-bus" },
  { name: "Taxi",       distance: 10,        icon: 'icon://fa-taxi' },
  { name: "Subway",     distance: 25,        icon: "icon://fa-subway" },
  { name: "Bicycle",    distance: 45,        icon: "icon://fa-bicycle" },
  { name: "Boat",       distance: 120,       icon: "icon://fa-ship" },
  { name: "Motorcycle", distance: 150,       icon: "icon://fa-motorcycle" },
  { name: "Car",        distance: 200,       icon: "icon://fa-car", },
  { name: "Train",      distance: 400,       icon: "icon://fa-train" },
  { name: "Airplane",   distance: 900,       icon: "icon://fa-plane" },
  { name: "Rocket",     distance: 100000000, icon: "icon://fa-rocket" }
];

function main() {
  initializeStylesForStaticElements();
  var previousChosenMiles = 0;
  onEvent("input-search", "click", function() {
	  var option = getText("input-trip-time");
	  var miles = keepNumbersInString(option);

  	if (miles == "") return;

  	if (miles == previousChosenMiles) return;
  	previousChosenMiles = miles;

  	var filteredVeichles = filterVeichlesUnderTripDistance(veichles, miles);
  	var display = generateDisplay(filteredVeichles);
  	$("output-vehicles").style = { color: "black" };
    display.render("output-vehicles");
  });
}

function filterVeichlesUnderTripDistance(veichles, tripDistance) {
  var filteredVeichles = [];
  for (var veichleIndex = 0; veichleIndex < veichles.length; veichleIndex++) {
    var veichle = veichles[veichleIndex];
    if (veichle.distance > tripDistance) {
      appendItem(filteredVeichles, veichle);
    }
  }
  return filteredVeichles;
}

function indexOf(list, item) {
  for (var i = 0; i < list.length; i++) {
    if (list[i] == item) {
      return i;
    }
  }
  return null;
}

function keepNumbersInString(string) {
  var numbers = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
  var resultString = "";
  for (var i = 0; i < string.length; i++) {
    if (indexOf(numbers, string[i]) !== null) {
      resultString += string[i];
    }
  }
  return resultString;
}

function generateDisplay(veichles) {
  var starRotateFn = function(event) {
    $(event.currentTargetId).style = {
      transform: "rotate(72deg)"
    };
  };
  var starDefaultFn = function(event) {
    $(event.currentTargetId).style = {
      transform: "rotate(0deg)"
    };
  };
  var containerHoveredFn = function(event) {
    $(event.currentTargetId).style = {
      "background-color": "antiquewhite"
    };
  };
  var containerDefaultFn = function(event) {
    $(event.currentTargetId).style = {
      "background-color": "white"
    };
  };

  var element = Compose.div("main", {
    style: {
      overflow: "auto",
      "max-height": "200px"
    }
  });
  for (var veichleIndex = 0; veichleIndex < veichles.length; veichleIndex++) {
    var veichle = veichles[veichleIndex];

    var veichleEntryDetails = [
      Compose.img(veichle.name + "-icon", {
        src: veichle.icon,
        width: 16
      }),
      Compose.span(veichle.name + "-label", {
        __inner__: veichle.name
      }),
      Compose.span(veichle.name + "-distance-label", {
        __inner__: "(For &lt;" + String(veichle.distance) + " miles)",
        style: {
          color: "darkgray",
          "margin-left": "auto"
        }
      }),
    ];
    if (veichleIndex == 0) appendItem(veichleEntryDetails, Compose.img(veichle.name + "-recommended", {
      src: "icon://fa-star",
      width: 16,
      style: {
        "margin-left": "auto",
        "transition": "transform 0.5s"
      },
      mouseover: starRotateFn,
      mouseout: starDefaultFn
    }));

    element.append(Compose.div(veichle.name + "-container", {
      style: {
        display: "flex",
        "align-items": "center",
        height: "100%",
        "border-radius": "2px",
      },
      mouseover: containerHoveredFn,
      mouseout: containerDefaultFn
    }).append(veichleEntryDetails)
    );
  }
  return element;
}

function initializeStylesForStaticElements() {
  var iconHoveredFn = function(event) {
    $(event.currentTargetId).style = {
      transform: "scale(1.1)"
    };
  };
  var iconDefaultFn = function(event) {
    $(event.currentTargetId).style = {
     transform: "scale(1.0)"
    };
  };

  $("input-search").style = {
    "border-top-left-radius": "0px",
    "border-bottom-left-radius": "0px"
  };
  $("input-trip-time").style = {
    "border-top-right-radius": "0px",
    "border-bottom-right-radius": "0px"
  };
  var iconIds = ["results-icon", "configuration-icon"];
  for (var i = 0; i < iconIds.length; i++) {
    var id = iconIds[i];
    $(id).style = { "transition": "transform 0.3s" };
    $(id).mouseover = iconHoveredFn;
    $(id).mouseout = iconDefaultFn;
  }

  $("input-search").mouseover = function(event) {
    $(event.currentTargetId).style = {
      "opacity": "0.90",
    };
  };
  $("input-search").mouseout = function(event) {
    $(event.currentTargetId).style = {
      "opacity": "1.0",
    };
  };

  $("title-image").click = function(event) {
    $(event.currentTargetId).style = {
      "transform": "translate(60px, -10px) rotate(45deg)",
      "transition": "transform 0.3s linear"
    };
    setTimeout(function() {
      $(event.currentTargetId).style = {
        "transform": "translate(90px, -10px) rotate(75deg)",
        "transition": "transform 0.12s linear"
      };
    }, 260);
    setTimeout(function() {
      $(event.currentTargetId).style = {
        "transform": "translate(170px, 10px) rotate(75deg)",
        "transition": "transform 0.32s linear"
      };
    }, 380);
    setTimeout(function() {
      $(event.currentTargetId).style = {
        "transform": "translate(330px, 0px) rotate(35deg)",
        "transition": "transform 0.68s linear"
      };
    }, 730);
    setTimeout(function() {
      $(event.currentTargetId).style = {
        "transform": "translate(0px, 0px) scale(0.1)",
        "opacity": "0.0",
       "transition": "transform 0.0s linear"
      };
    }, 2000);
    setTimeout(function() {
      $(event.currentTargetId).style = {
        "transform": "scale(1.0)",
        "opacity": "1.0",
        "transition": "transform 0.2s linear"
      };
    }, 2100);
  };
}

main();
