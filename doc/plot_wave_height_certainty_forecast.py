import json
import matplotlib.pyplot as plt
import datetime
import numpy as np

with open('gfs_wave_forecast_nws_wind.json') as f:
    wave_forecast = json.load(f)

# Get the wave heights and the wave_height_spread
date = [datetime.datetime.strptime((forecast['date'][:-1]), '%Y-%m-%dT%H:%M:%S') for forecast in wave_forecast]
wave_height = np.array([forecast['wave_summary']['wave_height']['value'] for forecast in wave_forecast])
wave_height_spread = np.array([(forecast['wave_height_spread']['value']/forecast['wave_summary']['wave_height']['value']) for forecast in wave_forecast])

plt.plot(date, wave_height, label='Wave Height')
plt.scatter(date, wave_height, c=wave_height_spread, cmap='coolwarm', vmin=0.0, vmax=1.0, label='Wave Height Spread')
plt.fill_between(date, wave_height - wave_height_spread * wave_height, wave_height + wave_height_spread * wave_height, alpha=0.2)
plt.show()