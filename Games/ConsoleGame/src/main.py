import os
import time
import threading
import random

class FORMAT:
    HEADER = '\033[95m'
    BLUE = '\033[94m'
    CYAN = '\033[96m'
    GREEN = '\033[92m'
    WARNING = '\033[93m'
    FAIL = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'

class User:
  money = 0.0
  has_water = False
  work_cooldown = 0
  pronoun = "Sir"
  entered_code = False

def manage_work_cd():
  while True:
    time.sleep(1)
    if User.work_cooldown > 0:
      User.work_cooldown -= 1
threading.Thread(target=manage_work_cd).start()

def order_water():
  os.system('clear')
  if User.money <= 1:
    print(f"{FORMAT.FAIL}Water costs $1, but you have ${User.money}.")
  else:
    print(f"{FORMAT.FAIL}You have acquired water, hooray!")
    print(f"{FORMAT.FAIL}Oh, and the code is water, literally.")
    User.has_water = True

  time.sleep(6)
  main_menu()

class JustWaitInput:
  wait = False
  input = ""

def just_wait():
  os.system('clear')
  print(f"{FORMAT.ENDC}You decided to wait...")
  time.sleep(5)
  print(f"{FORMAT.ENDC}Really, why are you waiting...")
  time.sleep(5)
  print(f"{FORMAT.ENDC}Unless you know the code, there's no reason to be here...")
  JustWaitInput.input = ""
  JustWaitInput.wait = True
  time.sleep(10)
  if JustWaitInput.input.lower() == 'water':
    if User.has_water:
      print(f"{FORMAT.ENDC}So you know the code ey!")
      print(f"{FORMAT.ENDC}I believe there's not much to do now.")
      User.entered_code = True
    else:
      print(f"{FORMAT.ENDC}You are not worthy of it's power.")
  else:
    print(f"{FORMAT.ENDC}Yeah, you know nothing.")
  JustWaitInput.wait = False
  time.sleep(3)
  main_menu()

def take_input():
  while True:
    if JustWaitInput.wait:
      response = input()
      JustWaitInput.input = response
threading.Thread(target=take_input).start()

def order_elsewise():
  os.system('clear')
  if not User.has_water:
    print(f"{FORMAT.FAIL}This store only sells water, strange right?")
    time.sleep(3)
    main_menu()

def outside_menu():
  os.system('clear')

  print(f"{FORMAT.CYAN}You Are Outside The Store:\n")
  print(f"{FORMAT.ENDC}{FORMAT.BOLD}Choices:")
  print(f"{FORMAT.WARNING}[1] Enter The Store (again)")
  print(f"{FORMAT.WARNING}[2] Leave The Store (even though you're already outside)")
  print(f"{FORMAT.WARNING}[3] Work")
  print(f"{FORMAT.WARNING}[4] Toggle Pronoun")
  print(f"{FORMAT.ENDC}Choose a option by typing in a number 1-{len(OutsideMenu.ValidInputs)}.")

  option_chosen = input(f"{FORMAT.GREEN}")
  try:
    option_chosen = int(option_chosen)
  except:
    main_menu("Type A Valid Option")
  else:
    if option_chosen in OutsideMenu.ValidInputs:
      OutsideMenu.Callbacks[option_chosen]()
    else:
      main_menu("Type A Valid Option")

class MainMenu:
  ValidInputs = [1,2,3,4]
  Callbacks = {1:order_water, 2:outside_menu, 3:just_wait, 4:order_elsewise}


def main_menu(message=None):
  os.system('clear')

  print(f"{FORMAT.CYAN}{User.pronoun}, do you want water?\n")
  print(f"{FORMAT.ENDC}{FORMAT.BOLD}Choices:")
  print(f"{FORMAT.WARNING}[1] Buy Water")
  print(f"{FORMAT.WARNING}[2] Leave The Store")
  print(f"{FORMAT.WARNING}[3] Wait")
  print(f"{FORMAT.WARNING}[4] Buy Something Else")
  print(f"{FORMAT.ENDC}Choose a option by typing in a number 1-4.")
  if message:
    print(f"{FORMAT.FAIL}{message}")

  option_chosen = input(f"{FORMAT.GREEN}")
  try:
    option_chosen = int(option_chosen)
  except:
    main_menu("Type A Valid Option")
  else:
    if option_chosen in MainMenu.ValidInputs:
      MainMenu.Callbacks[option_chosen]()
    else:
      main_menu("Type A Valid Option")

def leave_store_again():
  os.system('clear')
  if User.entered_code:
    print(f"{FORMAT.ENDC}You have somehow left the store again.")
    print(f"{FORMAT.GREEN}You have successfully completed the game!")
  else:
    print(f"{FORMAT.FAIL}How do you expect to leave the store again?")
    print(f"{FORMAT.ENDC}Unless... Nevermind.")
    time.sleep(6)
    outside_menu()

def work():
  os.system('clear')
  if User.work_cooldown == 0:
    amount = round(random.uniform(0.20, 0.45), 2)
    print(f"{FORMAT.ENDC}You worked and got ${amount}!")
    User.work_cooldown = 15
    User.money += amount
  else:
    print(f"{FORMAT.FAIL}You must be tired now... \n(wait {User.work_cooldown} seconds)")
  time.sleep(3)
  outside_menu()

def toggle_pronoun():
    os.system('clear')
    if User.pronoun == "Sir":
        User.pronoun = "Madam"
    else:
        User.pronoun = "Sir"
    print(f"{FORMAT.ENDC}{User.pronoun}, how is your day?")
    time.sleep(3)
    outside_menu()

class OutsideMenu:
  ValidInputs = [1,2,3,4]
  Callbacks = {1:main_menu, 2:leave_store_again, 3:work, 4:toggle_pronoun}

main_menu()
