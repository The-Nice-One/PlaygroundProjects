var Compose = new ComposeLibrary.init();

// extract certain function from ComposeLibrary to facilitate usage
var $ = Compose.$;
var Div = Compose.div;

var app = Div("my-div", {
  __inner__: "Hover over me and click!",
  title: "Open https://code.org",
  style: {
    display: "inline-block"
  },
  mouseover: function(event) {
    $(event.currentTargetId).style = {
      color: "blue",
      "text-decoration": "underline",
      cursor: "pointer"
    };
  },
  mouseout: function(event) {
    $(event.currentTargetId).style = {
      color: "black",
      "text-decoration": "none",
      cursor: "default"
    };
  },
  click: function() {
    open("https://code.org");
  }
});

app.render("screen1");