import discord
from discord.ext import commands, tasks
import datetime
import os
import sys
import json

bot = commands.Bot(command_prefix="!", strip_after_prefix=True, intents=discord.Intents.all())

data_dir = os.path.join(os.getcwd(), 'data')
storage_path = os.path.join(data_dir, 'storage.json')
with open(storage_path, "r") as file:
    global storage
    storage = json.load(file)

bot.command_prefix = storage["configuration"]["prefix"]

def save_storage():
    with open(storage_path, "w") as file:
        json.dump(storage, file, indent=4)

def reset_storage():
    global storage
    storage = {
        "points": {},
        "events": [],
        "log": [],
        "configuration": {
            "role": None,
            "emoji": ":star:",
            "point": "Reputation Point",
            "prefix": "!"
        }
    }
    save_storage()

@bot.event
async def on_ready():
    print('Reputation Bot is ready!')
    check_timed.start()

@bot.event
async def on_command_error(ctx, error):
    if isinstance(error, commands.errors.CommandNotFound):
        em = discord.Embed(title=":thinking: Are You Lost?", description=f"Type `{bot.command_prefix}help` to see a list of my commands!")
        await ctx.send(embed=em)
    else:
        raise error

@tasks.loop(seconds = 1)
async def check_timed():
    count = 0
    for tevents in storage["events"]:
        tdelta =  datetime.datetime(tevents['in'][0], tevents['in'][1], tevents['in'][2], tevents['in'][3], tevents['in'][4], tevents['in'][5], tevents['in'][6]) - datetime.datetime.now()
        if tdelta.total_seconds() < 1:
            try:
                storage["points"][str(tevents['user'])] += tevents['amount']
            except:
                storage["points"][str(tevents['user'])] = tevents['amount']
            storage["events"].pop(count)

            taction = f"**CHANGED** {tevents['amount']} {storage["configuration"]["emoji"]} `{storage["configuration"]["point"]}(s)` for {await bot.fetch_user(tevents['user'])}"
            add_logger(storage["configuration"]["emoji"] + " System", taction, [datetime.datetime.now().year, datetime.datetime.now().month, datetime.datetime.now().day, datetime.datetime.now().hour, datetime.datetime.now().minute, datetime.datetime.now().second, datetime.datetime.now().microsecond])
        count += 1

@bot.command(brief="Usage: !timed [@user] [minutes] (amount)", category="Management")
async def timed(ctx, user: discord.Member, min: int, amount: int = 1):
    errorm = discord.Embed(title=f"{storage["configuration"]["emoji"]} Uhh Oh!", description=f"You do not have `admin` permission or `{storage["configuration"]["role"]}` role!", color=ctx.author.color)
    successm = discord.Embed(title=f"{storage["configuration"]["emoji"]} Got It", description=f"I will give `{user}`, {amount} {storage["configuration"]["emoji"]} `{storage["configuration"]["point"]}(s)` in `{min}` minutes!", color=ctx.author.color)

    role = discord.utils.get(ctx.guild.roles, name=storage["configuration"]["role"])

    if ctx.author.guild_permissions.administrator or role in ctx.author.roles:
        t = datetime.datetime.now() + datetime.timedelta(minutes=min)
        when = [t.year, t.month, t.day, t.hour, t.minute, t.second, t.microsecond]
        storage["events"].append({'user': user.id, 'in': when, 'amount':amount})

        await ctx.send(embed=successm)

        taction = f"**TIMED** {user} to get {amount} {storage["configuration"]["emoji"]} `{storage["configuration"]["point"]}(s)` in `{min}` minutes"

        add_logger(ctx.author.name + '#' +  ctx.author.discriminator, taction, [datetime.datetime.now().year, datetime.datetime.now().month, datetime.datetime.now().day, datetime.datetime.now().hour, datetime.datetime.now().minute, datetime.datetime.now().second, datetime.datetime.now().microsecond])
    else:
        await ctx.send(embed=errorm)

@bot.command(brief="Usage: !profile", category="Social")
async def profile(ctx):
    if str(ctx.author.id) not in storage["points"]:
        storage["points"][str(ctx.author.id)] = 0

    my_points = storage["points"][str(ctx.author.id)]

    profile_embed = discord.Embed(title=ctx.author, description=f"ID: `{ctx.author.id}`\n{storage["configuration"]["emoji"]} You have {my_points} {storage["configuration"]["emoji"]} `{storage["configuration"]["point"]}(s)`", color=ctx.author.color)
    profile_embed.set_thumbnail(url=ctx.author.avatar.url)

    my_timed = []
    for event in storage["events"]:
        if event['user'] == ctx.author.id:
            my_timed.append(event)

    if len(my_timed) > 0:
        string = ""

        for mtevents in my_timed:
            time1 = datetime.datetime(mtevents['in'][0], mtevents['in'][1], mtevents['in'][2], mtevents['in'][3], mtevents['in'][4], mtevents['in'][5], mtevents['in'][6])
            time2 = datetime.datetime.now()
            mtdelta = time1 - time2
            seconds = mtdelta.total_seconds()
            hours = seconds // 3600
            minutes = (seconds % 3600) // 60

            string += f":timer: {mtevents['amount']} {storage["configuration"]["emoji"]} `{storage["configuration"]["point"]}(s)` in `{hours}` hours and `{minutes}` minutes!\n"
    else:
        string = f"You have no timed `{storage["configuration"]["point"]}(s)` given to you!"

    profile_embed.add_field(name="Timed Events", value=string)
    await ctx.send(embed=profile_embed)


@bot.command(brief="Usage: !log", category="Management")
async def log(ctx):
    lem = discord.Embed(title="Logging System", description="Listing the last 15 actions | :one: = Most recent action", color=ctx.author.color)
    count = 0
    evorder = [':one:', ':two:', ':three:', ':four:', ':five:', ':six:', ':seven:', ':eight:', ':nine:', ':one::zero:', ':one::one:', ':one::two:', ':one::three:', ':one::four:', ':one::five:']
    for actions in reversed(storage["log"]):
        delta = datetime.datetime.now() - datetime.datetime(actions['time'][0], actions['time'][1], actions['time'][2], actions['time'][3], actions['time'][4], actions['time'][5], actions['time'][6])
        seconds = delta.total_seconds()
        hours = seconds // 3600
        minutes = (seconds % 3600) // 60

        lem.add_field(name=f"{evorder[count]} Action By: {actions['user']}", value=f"{actions['action']}\nAction done `{round(hours)}` hour(s) and `{round(minutes)}` minute(s) ago")

        count += 1

    await ctx.send(embed=lem)

def add_logger(user, action, time):
    if len(storage["log"]) > 14:
        storage["log"].pop(-1)

    storage["log"].append({'user':user, 'action':action, 'time':time})
    save_storage()

@bot.command(brief="Usage: !rep [@user] (amount)", category="Management")
async def rep(ctx, user: discord.Member, eamount: int = 1):
    successm = discord.Embed(title=f"{storage["configuration"]["emoji"]} `{storage["configuration"]["point"]}(s)` Given", description=f"I have given {eamount} {storage["configuration"]["emoji"]} `{storage["configuration"]["point"]}(s)` To {user}!", color=ctx.author.color)
    errorm = discord.Embed(title=f"{storage["configuration"]["emoji"]} Uhh Oh!", description=f"You do not have `admin` permission or `{storage["configuration"]["role"]}` role!", color=ctx.author.color)
    if isinstance(user, discord.Member):
        uid = str(user.id)
    else:
        uid = user

    role = discord.utils.get(ctx.guild.roles, name=storage["configuration"]["role"])

    if role in ctx.author.roles or ctx.author.guild_permissions.administrator:
        if uid not in storage["points"]:
            storage["points"][uid] = eamount
        else:
            storage["points"][uid] += eamount
        taction = f"**CHANGED** {eamount} {storage["configuration"]["emoji"]} `{storage["configuration"]["point"]}(s)` for {user}"

        add_logger(ctx.author.name + '#' +  ctx.author.discriminator, taction, [datetime.datetime.now().year, datetime.datetime.now().month, datetime.datetime.now().day, datetime.datetime.now().hour, datetime.datetime.now().minute, datetime.datetime.now().second, datetime.datetime.now().microsecond])

        await ctx.send(embed=successm)
    else:
        await ctx.send(embed=errorm)


@bot.command(brief="Usage: !setrole [@role]", category="Configuration")
async def setrole(ctx, role: discord.Role):
    arole = discord.utils.get(ctx.guild.roles, name=storage["configuration"]["role"])

    errorm = discord.Embed(title=f"{storage["configuration"]["emoji"]} Uhh Oh!", description=f"You do not have `admin` permission or `{storage["configuration"]["role"]}` role!", color=ctx.author.color)
    successm = discord.Embed(title=f"{storage["configuration"]["emoji"]} Permission Changed", description=f"Users who have `admin` permission or have the `{role.name}` role will be able to give `{storage["configuration"]["point"]}(s)`", color=ctx.author.color)

    if ctx.author.guild_permissions.administrator or arole in ctx.author.roles:
        storage["configuration"]["role"] = role.name

        taction = f"**SET** `{role.name}` as the role needed to give {storage["configuration"]["emoji"]} `{storage["configuration"]["point"]}(s)`"

        add_logger(ctx.author.name + '#' +  ctx.author.discriminator, taction, [datetime.datetime.now().year, datetime.datetime.now().month, datetime.datetime.now().day, datetime.datetime.now().hour, datetime.datetime.now().minute, datetime.datetime.now().second, datetime.datetime.now().microsecond])

        await ctx.send(embed=successm)
    else:
        await ctx.send(embed=errorm)

@bot.command(brief="Usage: !setemoji [emoji]", category="Configuration")
async def setemoji(ctx, emoji):
    arole = discord.utils.get(ctx.guild.roles, name=storage["configuration"]["role"])

    errorm = discord.Embed(title=f"{storage["configuration"]["emoji"]} Uhh Oh!", description=f"You do not have `admin` permission or `{storage["configuration"]["role"]}` role!", color=ctx.author.color)
    successm = discord.Embed(title=f"{storage["configuration"]["emoji"]} Emoji Changed", description=f"The emoji used to represent `{storage["configuration"]["point"]}(s)` is now {emoji}", color=ctx.author.color)

    if ctx.author.guild_permissions.administrator or arole in ctx.author.roles:
        storage["configuration"]["emoji"] = emoji

        taction = f"**SET** `{emoji}` as the emoji for `{storage["configuration"]["point"]}(s)`"

        add_logger(ctx.author.name + '#' +  ctx.author.discriminator, taction, [datetime.datetime.now().year, datetime.datetime.now().month, datetime.datetime.now().day, datetime.datetime.now().hour, datetime.datetime.now().minute, datetime.datetime.now().second, datetime.datetime.now().microsecond])

        await ctx.send(embed=successm)
    else:
        await ctx.send(embed=errorm)

@bot.command(brief="Usage: !setpoint [point]", category="Configuration")
async def setpoint(ctx, *point):
    point = " ".join(point)
    arole = discord.utils.get(ctx.guild.roles, name=storage["configuration"]["role"])

    errorm = discord.Embed(title=f"{storage["configuration"]["emoji"]} Uhh Oh!", description=f"You do not have `admin` permission or `{storage["configuration"]["role"]}` role!", color=ctx.author.color)
    successm = discord.Embed(title=f"{storage["configuration"]["emoji"]} Point(s) Name Changed", description=f"`{point}` will now be used to refer to points.", color=ctx.author.color)

    if ctx.author.guild_permissions.administrator or arole in ctx.author.roles:
        storage["configuration"]["point"] = point

        taction = f"**SET** `{point}` as the point(s) name `"

        add_logger(ctx.author.name + '#' +  ctx.author.discriminator, taction, [datetime.datetime.now().year, datetime.datetime.now().month, datetime.datetime.now().day, datetime.datetime.now().hour, datetime.datetime.now().minute, datetime.datetime.now().second, datetime.datetime.now().microsecond])

        await ctx.send(embed=successm)
    else:
        await ctx.send(embed=errorm)

@bot.command(brief="Usage: !setprefix [prefix]", category="Configuration")
async def setprefix(ctx, prefix):
    arole = discord.utils.get(ctx.guild.roles, name=storage["configuration"]["role"])

    errorm = discord.Embed(title=f"{storage["configuration"]["emoji"]} Uhh Oh!", description=f"You do not have `admin` permission or `{storage["configuration"]["role"]}` role!", color=ctx.author.color)
    successm = discord.Embed(title="Prefix Changed", description=f"`{prefix}` is now the prefix for commands", color=ctx.author.color)

    if ctx.author.guild_permissions.administrator or arole in ctx.author.roles:
        storage["configuration"]["prefix"] = prefix
        bot.command_prefix = storage["configuration"]["prefix"]

        taction = f"**SET** `{prefix}` as the prefix for commands`"

        add_logger(ctx.author.name + '#' +  ctx.author.discriminator, taction, [datetime.datetime.now().year, datetime.datetime.now().month, datetime.datetime.now().day, datetime.datetime.now().hour, datetime.datetime.now().minute, datetime.datetime.now().second, datetime.datetime.now().microsecond])

        await ctx.send(embed=successm)
    else:
        await ctx.send(embed=errorm)

@bot.command(brief="Usage: !reset [all/@user]", category="Management")
async def reset(ctx, thing):

    role = discord.utils.get(ctx.guild.roles, name=storage["configuration"]["role"])
    errorm = discord.Embed(title=f"{storage["configuration"]["emoji"]} Uhh Oh!", description=f"You do not have `admin` permission or `{storage["configuration"]["role"]}` role!", color=ctx.author.color)

    if ctx.author.guild_permissions.administrator or role in ctx.author.roles:
        if thing.lower() == 'all':
            storage["points"] = {}

            successm = discord.Embed(title=f"{storage["configuration"]["emoji"]} Reset", description=f"{storage["configuration"]["emoji"]} `{storage["configuration"]["point"]}(s)` for all users have been reset", color=ctx.author.color)

            taction = f"**RESET** everyone's {storage["configuration"]["emoji"]} `{storage["configuration"]["point"]}(s)`"

            add_logger(ctx.author.name + '#' +  ctx.author.discriminator, taction, [datetime.datetime.now().year, datetime.datetime.now().month, datetime.datetime.now().day, datetime.datetime.now().hour, datetime.datetime.now().minute, datetime.datetime.now().second, datetime.datetime.now().microsecond])

            await ctx.send(embed=successm)

        else:

            user = thing.replace("<","")
            user = user.replace(">","")
            user = user.replace("@","")
            user = user.replace("!","")
            user = user.strip()

            del storage["points"][str(user)]

            successm = discord.Embed(title=f"{storage["configuration"]["emoji"]} Reset", description=f"{storage["configuration"]["emoji"]} `{storage["configuration"]["point"]}(s)` for {await bot.fetch_user(user)} have been reset", color=ctx.author.color)

            taction = f"**RESET** {await bot.fetch_user(user)}'s {storage["configuration"]["emoji"]} `{storage["configuration"]["point"]}(s)`"

            add_logger(ctx.author.name + '#' +  ctx.author.discriminator, taction, [datetime.datetime.now().year, datetime.datetime.now().month, datetime.datetime.now().day, datetime.datetime.now().hour, datetime.datetime.now().minute, datetime.datetime.now().second, datetime.datetime.now().microsecond])

            await ctx.send(embed=successm)
    else:
        await ctx.send(embed=errorm)



@bot.command(brief="Usage: !top", category="Social")
async def top(ctx):

    leaderboard_embed = discord.Embed(title=f'{storage["configuration"]["emoji"]} {storage["configuration"]["point"]} Leaderboard', color=ctx.author.color)

    sorted_users = dict(sorted(storage["points"].items(), key=lambda item: item[1], reverse=True))

    string = ""
    num = 0
    for user in sorted_users:
        num += 1
        if num == 1:
            emoji = ':first_place:'
        elif num == 2:
            emoji = ':second_place:'
        elif num == 3:
            emoji = ':third_place:'
        else:
            emoji = ':medal:'

        string += f"{emoji} {await bot.fetch_user(user)}: **{sorted_users[user]}** {storage["configuration"]["emoji"]} `{storage["configuration"]["point"]}(s)`\n"
    if string == "":
        string = f"No one has atleast 1 {storage["configuration"]["emoji"]} `{storage["configuration"]["point"]}(s)`"
    leaderboard_embed.add_field(name=f"Sorted {len(sorted_users)} users", value=string)
    await ctx.send(embed=leaderboard_embed)

def run():
    bot.run(sys.argv[1])
