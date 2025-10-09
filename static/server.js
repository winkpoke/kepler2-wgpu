const cors = require("cors");
const express = require("express");
const { Worker } = require("worker_threads");
const path = require("path");

const app = express();
app.use(express.json());
app.use(cors());

// ======================
// API: 调用 DLL 重建
// ======================
app.post("/api/run-recon", (req, res) => {
  console.log("进入 /api/run-recon");

  // 开一个 worker 执行 DLL 调用
  const worker = new Worker(path.resolve(__dirname, "worker.js"), {
    workerData: req.body, // 将请求体传给 worker
  });

  // Worker 成功返回
  worker.once("message", (result) => {
    console.log("准备返回给前端:", result);
    console.log("headersSent:", res.headersSent);
    res.status(200).json({
      success: result.success,
      code: result.code,
      output: result.output,
    });
  });

  // Worker 出错
  worker.once("error", (err) => {
    console.error("Worker 出错:", err);
    if (!res.headersSent) {
      res.status(500).json({ error: err.message });
    }
  });
});

// ======================
// 启动服务
// ======================
app.listen(3000, () => {
  console.log("✅ Server listening at http://localhost:3000");
});
