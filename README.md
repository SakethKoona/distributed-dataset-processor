# Distributed Dataset Processor

A lightweight and efficient **distributed dataset processing service** designed to automate mundane preprocessing tasks for image ML workflows.  

It supports common operations such as **grayscale conversion, data augmentation, resizing**, and more, allowing you to streamline dataset preparation in a scalable and fault-tolerant manner.

---

## 🧱 Features

- Distributed and asynchronous task processing using a **producer-consumer architecture**  
- Middleware: **Kafka** for task messaging  
- Database: **MongoDB** for metadata and workflow tracking  
- Supports **idempotent retries** and **concurrent processing**  
- Handles routine image preprocessing automatically (grayscale, resize, noise, augmentation)  

---

## 🚀 Example Usage

A sample request for processing a dataset:

```bash
-d '{
  "dataset_key": "images/kanagawa.zip",
  "operations": [
    { "Resize": { "scaling_factor": 0.5 } },
    "GrayScale",
    { "Noise": { "noise_level": 0.1 } }
  ]
}'
