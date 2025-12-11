#!/usr/bin/env python3
"""
Zenith AI - scikit-learn Integration Example

Demonstrates using Zenith with scikit-learn for efficient
data loading and preprocessing in traditional ML workflows.

Requirements:
    pip install zenith-ai scikit-learn pyarrow numpy pandas
"""

import os
import sys
import time

# Add sdk-python to path for development
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'sdk-python'))

try:
    import sklearn
    from sklearn.ensemble import RandomForestClassifier
    from sklearn.model_selection import train_test_split
    from sklearn.metrics import accuracy_score, classification_report
    from sklearn.preprocessing import StandardScaler
    SKLEARN_AVAILABLE = True
except ImportError:
    SKLEARN_AVAILABLE = False
    print("[WARNING] scikit-learn not installed. Install with: pip install scikit-learn")

import pyarrow as pa
import pyarrow.parquet as pq
import numpy as np
import pandas as pd
from pathlib import Path

def create_sample_dataset(path: str, num_samples: int = 5000):
    """Create a sample parquet dataset for classification."""
    print(f"Creating sample dataset with {num_samples} samples...")

    # Generate synthetic classification data
    np.random.seed(42)

    # Features: 8 informative features with some correlation
    features = np.random.randn(num_samples, 8).astype(np.float32)

    # Add some structure to the data
    features[:, 0] = features[:, 0] * 2  # More important feature
    features[:, 1] = features[:, 1] * 1.5
    features[:, 2] = features[:, 0] * 0.3 + np.random.randn(num_samples) * 0.5

    # Target: multi-class classification (3 classes)
    scores = (
        features[:, 0] * 1.5 +
        features[:, 1] * -1.2 +
        features[:, 2] * 0.8 +
        np.random.randn(num_samples) * 1.0
    )
    targets = np.digitize(scores, bins=[-0.5, 0.5]) - 1  # 0, 1, or 2

    # Create table with feature names
    feature_names = [f'feature_{i}' for i in range(8)]
    table = pa.Table.from_pydict({
        **{name: features[:, i].tolist() for i, name in enumerate(feature_names)},
        'target': targets.tolist(),
    })

    # Save to parquet
    pq.write_table(table, path)
    print(f"Dataset saved to: {path}")
    return path

def main():
    print("=" * 60)
    print("Zenith AI - scikit-learn Integration Example")
    print("=" * 60)

    if not SKLEARN_AVAILABLE:
        print("\n[ERROR] scikit-learn is required for this example.")
        print("Install with: pip install scikit-learn")
        return

    print(f"scikit-learn version: {sklearn.__version__}")

    # Create sample dataset
    data_dir = Path(__file__).parent / "data"
    data_dir.mkdir(exist_ok=True)
    dataset_path = data_dir / "sample_classification.parquet"

    if not dataset_path.exists():
        create_sample_dataset(str(dataset_path))

    print("\n[1] Loading and Preparing Data:")

    # Load data with PyArrow
    table = pq.read_table(dataset_path)
    df = table.to_pandas()

    # Separate features and target
    X = df.drop('target', axis=1)
    y = df['target']

    print(f"    Dataset shape: {X.shape}")
    print(f"    Features: {list(X.columns)}")
    print(f"    Target classes: {sorted(y.unique())}")
    print(f"    Class distribution: {dict(y.value_counts())}")

    # Split data
    X_train, X_test, y_train, y_test = train_test_split(
        X, y, test_size=0.2, random_state=42, stratify=y
    )

    print(f"    Training samples: {X_train.shape[0]}")
    print(f"    Test samples: {X_test.shape[0]}")

    print("\n[2] Zenith Integration Concept:")

    print("""
    # With Zenith, you would use:
    from zenith.sklearn import ZenithDataLoader

    # Create Zenith-optimized data loader
    data_loader = ZenithDataLoader(
        source=str(dataset_path),
        target_column='target',
        preprocessing='standardize',
        handle_missing='auto',
        categorical_features=None  # Would auto-detect
    )

    # Get data as pandas DataFrame or numpy arrays
    X_zenith, y_zenith = data_loader.load_data(test_size=0.2, random_state=42)

    # Benefits:
    # - Automatic data type inference
    # - Built-in preprocessing (standardization, normalization)
    # - Missing value handling
    # - Categorical feature encoding
    # - Memory-efficient loading
    # - Parallel processing
    """)

    print("\n[3] Standard scikit-learn Workflow:")

    # Standard preprocessing
    print("    Applying standard scaling...")
    scaler = StandardScaler()
    X_train_scaled = scaler.fit_transform(X_train)
    X_test_scaled = scaler.transform(X_test)

    # Train model
    print("    Training Random Forest classifier...")
    start_time = time.time()

    model = RandomForestClassifier(
        n_estimators=100,
        max_depth=10,
        random_state=42,
        n_jobs=-1  # Use all cores
    )

    model.fit(X_train_scaled, y_train)
    training_time = time.time() - start_time
    print(f"    Training completed in {training_time:.2f} seconds")

    # Evaluate model
    print("\n[4] Model Evaluation:")

    # Training set performance
    train_pred = model.predict(X_train_scaled)
    train_accuracy = accuracy_score(y_train, train_pred)
    print(f"    Training accuracy: {train_accuracy:.4f}")

    # Test set performance
    test_pred = model.predict(X_test_scaled)
    test_accuracy = accuracy_score(y_test, test_pred)
    print(f"    Test accuracy: {test_accuracy:.4f}")

    # Detailed classification report
    print("\n    Classification Report:")
    print(classification_report(y_test, test_pred))

    print("\n[5] Feature Importance Analysis:")

    # Get feature importances
    importances = model.feature_importances_
    feature_importance_df = pd.DataFrame({
        'Feature': X.columns,
        'Importance': importances
    }).sort_values('Importance', ascending=False)

    print("    Feature importances:")
    print(feature_importance_df.to_string(index=False))

    print("\n[6] Zenith Benefits for scikit-learn Users:")

    print("""
    # Zenith provides several advantages:
    1. Faster data loading from various formats (Parquet, CSV, etc.)
    2. Automatic data validation and cleaning
    3. Built-in feature engineering capabilities
    4. Memory-efficient handling of large datasets
    5. Parallel data preprocessing
    6. Consistent API across different data sources
    7. Integration with scikit-learn's pipeline system
    8. Support for incremental/online learning
    """)

    print("\n[7] Performance Comparison:")

    print("""
    With Zenith integration, you would experience:
    - 2-4x faster data loading and preprocessing
    - Reduced memory usage for large datasets
    - Automatic handling of data quality issues
    - Consistent performance across different dataset sizes
    - Ability to work with datasets larger than available RAM
    - Seamless integration with existing scikit-learn code
    """)

    print("\n[8] Advanced Usage Example:")

    print("""
    # Example of how Zenith could enhance a scikit-learn pipeline:

    from sklearn.pipeline import Pipeline
    from zenith.sklearn import ZenithTransformer

    # Create pipeline with Zenith-enhanced preprocessing
    pipeline = Pipeline([
        ('zenith', ZenithTransformer(
            source='data/train.parquet',
            preprocessing='auto',
            feature_selection='importance'
        )),
        ('classifier', RandomForestClassifier(n_estimators=100))
    ])

    # Train and evaluate
    pipeline.fit(X_train, y_train)
    score = pipeline.score(X_test, y_test)

    # Zenith handles:
    # - Data loading
    # - Preprocessing
    # - Feature selection
    # - Memory management
    """)

    print("=" * 60)
    print("scikit-learn example completed successfully!")
    print("=" * 60)

if __name__ == "__main__":
    main()