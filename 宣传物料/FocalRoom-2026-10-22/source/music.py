"""Original ambient score and subtle edit cues, generated locally; no sampled music."""
from pathlib import Path
import numpy as np
import wave
SR=48000; DURATION=32
out=Path(__file__).resolve().parents[1]/"assets/score.wav"
audio=np.zeros((SR*DURATION,2),dtype=np.float64)
def add(at,frequency,duration,level,pan=0,kind="pluck"):
 n=min(int(duration*SR),len(audio)-int(at*SR))
 if n<=0:return
 t=np.arange(n)/SR
 if kind=="pad":
  env=np.minimum(t/.6,1)*np.minimum((duration-t)/1.2,1)*np.exp(-t/10)
  tone=(np.sin(2*np.pi*frequency*t)+.18*np.sin(2*np.pi*frequency*2*t))*.6
 else:
  env=(1-np.exp(-t/.008))*np.exp(-t/1.1)*np.minimum((duration-t)/.25,1)
  tone=np.sin(2*np.pi*frequency*t)+.22*np.sin(2*np.pi*frequency*2.003*t)+.06*np.sin(2*np.pi*frequency*4*t)
 a=int(at*SR);sig=tone*env*level
 audio[a:a+n,0]+=sig*np.sqrt((1-pan)/2)
 audio[a:a+n,1]+=sig*np.sqrt((1+pan)/2)
chords=[[146.832,220,261.626,329.628],[116.541,174.614,233.082,293.665],[130.813,196,261.626,329.628],[130.813,164.814,196,293.665]]
beat=60/88
for bar in range(11):
 at=bar*4*beat;chord=chords[bar%4]
 for j,f in enumerate(chord):add(at,f,4.8,.043,(-1+j*2/3)*.35,"pad")
 for step in range(8):
  ti=at+step*beat/2
  if ti<29.4:add(ti,chord[[0,2,1,3,2,1,3,2][step]]*2,2.4,.055 if step%2==0 else .035,np.sin(step)*.4)
 if at<28:add(at,chord[0]/2,2.2,.09,0,"pad")
for at in [4.8,10.8,16.8,22.8,27.8]:
 add(at,880,1.2,.028,-.25);add(at+.065,1320,.9,.015,.25)
for i,delay in enumerate([.17,.31,.47]):
 d=int(delay*SR);audio[d:]+=audio[:-d,::-1]*[.12,.08,.035][i]
t=np.arange(len(audio))/SR
audio*=(np.minimum(t/1.1,1)*np.minimum((DURATION-t)/2.5,1))[:,None]
audio=np.tanh(audio*1.35)
audio*=.75/max(np.max(np.abs(audio)),.001)
with wave.open(str(out),"wb") as w:
 w.setnchannels(2);w.setsampwidth(2);w.setframerate(SR);w.writeframes((audio*32767).astype("<i2").tobytes())
print(out)
