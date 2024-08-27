REM General Bots: Your Prompt Engineering Gets Done. 
PARAM  location AS "The city and state, e.g. San Francisco, CA" 
PARAM unit AS "celsius", "fahrenheit" 
DESCRIPTION "Get the current weather in a given location" 

REM 'TODO: Call your favorite wheather API here and return it to LLM.

weather_info = NEW OBJECT 
weather_info.location = location 
weather_info. Temperature = "72" 
weather_info. Unit = unit 
weather_info. forecast =  ["sunny", "windy"] 

RETURN weather_info 