import torch
import torch.nn as nn
import torch.optim as optim
from torch.utils.data import Dataset, DataLoader
import numpy as np
import pandas as pd


class DatasetReader(Dataset):
    def __init__(self, filepath):
        data = pd.read_csv(filepath, header=None)
        self.X = data.iloc[:, :-1].values.astype("float32")
        self.y = data.iloc[:, -1].values.astype("float32")

        self.mean = self.X.mean(axis=0)
        self.std = self.X.std(axis=0)
        self.X = ((self.X - self.mean) / self.std).astype("float32")

    def __len__(self):
        return len(self.y)

    def __getitem__(self, idx):
        return self.X[idx], self.y[idx]


class Model(nn.Module):
    def __init__(self, ft, hl):
        super(Model, self).__init__()
        self.net = nn.Sequential(
            nn.Linear(ft, hl),
            nn.ReLU(),
            nn.Linear(hl, 1),
        )

    def forward(self, x):
        return self.net(x)


def train_model(csv_file, epochs, batch_size, lr):
    dataset = DatasetReader(csv_file)
    dataloader = DataLoader(
        dataset, batch_size=batch_size, shuffle=True, num_workers=2, pin_memory=True
    )

    model = Model(ft=dataset.X.shape[1], hl=32)
    criterion = nn.BCEWithLogitsLoss()
    optimizer = optim.Adam(model.parameters(), lr=lr)

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    model.to(device)

    for epoch in range(epochs):
        loss = 0
        correct, total = 0, 0

        for X_batch, y_batch in dataloader:
            X_batch, y_batch = X_batch.to(device), y_batch.to(device).unsqueeze(1)

            optimizer.zero_grad()
            outputs = model(X_batch)
            loss = criterion(outputs, y_batch)
            loss.backward()
            optimizer.step()

            loss += loss.item() * y_batch.size(0)
            preds = (outputs >= 0.0).float()
            correct += (preds == y_batch).sum().item()
            total += y_batch.size(0)

        print(
            f"[Epoch {epoch + 1:02d}/{epochs}] "
            f"Loss: {loss / total:.8f} | "
            f"Accuracy: {100 * correct / total:.2f}%"
        )

    return model, dataset.mean, dataset.std


def pretty_print(label, arr):
    if isinstance(arr, torch.Tensor):
        data = arr.detach().cpu().numpy()
    else:
        data = np.array(arr)

    print(f"\n{label} shape: {data.shape}")
    print(np.array2string(data, formatter={"float_kind": lambda x: f"{x:.8f}"}))


if __name__ == "__main__":
    model, mean, std = train_model("data.csv", epochs=20, batch_size=8192, lr=0.001)

    pretty_print("Normalization mean", mean)
    pretty_print("Normalization std", std)

    for name, param in model.named_parameters():
        pretty_print(name, param)
