Source: https://github.com/MalumaDev/VAEEDOF/blob/78cf9ba10b9e80cef3953cb688971b34c4dcb11e/README.md
Accessed: 2026-11-15


# 🧠 VAEDOF: Extended Depth of Field with Variational Autoencoders

This repository contains code for training and evaluating **VAEEDOF**, a Variational Autoencoder-based model for generating extended depth of field (EDOF) images.

## 🚀 Running the Experiment (VAEEDOF)

To launch the experiment using multiple GPUs:

```bash
accelerate launch \
  --config_file accelerate_configs/2_gpus.config \
  --gpu_ids 1,2 \
  main.py --config configs/VAEEDOF.yaml
```

✅ You can modify any parameters — including model architecture, training settings, or dataset paths — directly in the YAML config file.

---


## 📦 Dependencies

Install the required Python packages:

```bash
pip install -r requirements.txt
```

---

🤗 Pretrained Model on Hugging Face

You can download our model and models used for comparison at:

🔗 [https://huggingface.co/your-username/vaedof](https://huggingface.co/MalumaDev/VAEAEDOF)


---

📄 Citation

If you use VAEEDOF in your research, please cite the following paper:

```
@article{piano2025addressing,
  title={Addressing the Depth-of-Field Constraint: A New Paradigm for High Resolution Multi-Focus Image Fusion},
  author={Piano, Luca and Huanwen, Peng and Bilcu, Radu Ciprian},
  journal={arXiv preprint arXiv:2510.19581},
  year={2025}
}
```

---

## 📬 Contact

For questions or contributions, feel free to open an issue or a pull request.

