var uri = 'http://' + location.host + '/roll';

var sse = new EventSource(uri);

var user_id;

function roll(data) {
  console.log(data);
  var dice_roll = JSON.parse(data);
  var line = document.createElement('p');
  line.innerText = dice_roll.name + ' rolled a ' + dice_roll.roll_result + ' (W' + dice_roll.dice_sides + ')';
  diceLog.insertBefore(line, diceLog.firstChild);
}

sse.onopen = function() {
  diceLog.innerHTML = "<p><em>Connected!</em></p>";
}

sse.addEventListener("roll", function(msg) {
  user_id = msg.data;
});

sse.onmessage = function(msg) {
  roll(msg.data);
};

send.onclick = function() {
  var char_name = charname.value;
  var dice_sides = dicesides.value;
  var xhr = new XMLHttpRequest();
  xhr.open("POST", uri + '/' + char_name + '/' + dice_sides, true);
  xhr.send();
};
