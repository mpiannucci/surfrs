import json
import matplotlib.pyplot as plt
import datetime
import numpy as np

with open('gfs_wave_forecast_nws_wind.json') as f:
    wave_forecast = json.load(f)

# Get the wave heights and the wave_height_spread
date = [datetime.datetime.strptime(
    (forecast['date'][:-1]), '%Y-%m-%dT%H:%M:%S') for forecast in wave_forecast]
wave_height = np.array(
    [forecast['wave_summary']['wave_height']['value'] for forecast in wave_forecast])
wave_height_spread = np.array(
    [forecast['wave_height_spread']['value'] for forecast in wave_forecast])
wave_height_mean = np.array(
    [forecast['wave_height_mean']['value'] for forecast in wave_forecast])
normalized_spread = 1 - (wave_height_spread / wave_height_mean)

print(f"{np.min(normalized_spread[0:48])} - {np.max(normalized_spread[0:48])}")

plt.plot(date, wave_height, c='grey', alpha=0.25, linewidth=5, label='GFS Wave Height')
plt.scatter(date, wave_height, c=normalized_spread, cmap='Spectral', vmin=0.0, vmax=1.0, label='GFS Ensemble Wave Height Uncertainty')

plt.fill_between(date, wave_height_mean - wave_height_spread * 0.5,
                 wave_height_mean + wave_height_spread * 0.5, alpha=0.2, label='GFS Ensemble Mean Wave Height')

plt.xlabel('Date')
plt.ylabel('Wave Height (ft)')
plt.legend()
plt.grid()

plt.show()
