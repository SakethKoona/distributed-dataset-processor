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
```

## 🔄 Workflow
1. Dataset is ingested client-side into an S3 bucket.

2. Tasks are sent via Kafka to the distributed workers.

3. Each operation is applied asynchronously, with retries if failures occur.

4. Dependency-aware task tracking using MongoDB.

## ⚙️ Future Plans

Web frontend for monitoring workflows and datasets

More advanced augmentation and preprocessing modules

Enhanced monitoring and logging for distributed processing



---

## ⚙️ Future Plans

- Web frontend for monitoring workflows and datasets  
- More advanced augmentation and preprocessing modules  
- Enhanced monitoring and logging for distributed processing  

---

## 💡 Notes

- Designed to **reduce manual preprocessing** in machine learning workflows.  
- Fully **asynchronous and concurrent**, ensuring efficient resource usage.  
- Works with large datasets stored in S3 or compatible object stores.  
- **Docker Compose** allows easy deployment and orchestration of all service components.

---

## 🚧 Project Status

This project is **still in progress**. While the core backend and worker functionality is implemented and running, some features and the frontend are **not yet complete**. The repository will be updated continuously as development progresses.

---

## 📦 How to Run (Optional Section)

1. Make sure you have **Docker** and **Docker Compose** installed.  
2. Clone the repository:
```bash
git clone https://github.com/YOUR-USERNAME/ImageProcessor.git
cd ImageProcessor
docker-compose up --build
```
