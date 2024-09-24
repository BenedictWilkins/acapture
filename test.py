import acapture
import matplotlib.pyplot as plt


targets = acapture.get_targets()
for target in targets:
    print(target)

env = acapture.Environment()
obs, info = env.reset()  # this will stop and restart the capture
obs, *_ = env.step(None)

env.close()

plt.imshow(obs)
plt.show()
