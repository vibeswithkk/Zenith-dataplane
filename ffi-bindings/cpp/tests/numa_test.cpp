/**
 * Zenith NUMA Backend Unit Tests
 *
 * Tests for the C++ NUMA backend FFI layer.
 * Requires Google Test and libnuma.
 *
 * Build:
 *   cd ffi-bindings/cpp && mkdir build && cd build
 *   cmake .. -DBUILD_TESTS=ON && make
 *   ./numa_test
 */

#include "../zenith_numa.h"
#include <gtest/gtest.h>

class NumaBackendTest : public ::testing::Test {
protected:
  void SetUp() override {
    // Initialize NUMA subsystem
    init_result = zenith_numa_init();
  }

  void TearDown() override { zenith_numa_cleanup(); }

  int32_t init_result = ZENITH_NUMA_ERR_UNAVAILABLE;
};

// Initialization tests
TEST_F(NumaBackendTest, InitSucceedsOrUnavailable) {
  // Either NUMA is available and init succeeds, or it's unavailable
  EXPECT_TRUE(init_result == ZENITH_NUMA_OK ||
              init_result == ZENITH_NUMA_ERR_UNAVAILABLE);
}

TEST_F(NumaBackendTest, AvailableMatchesInit) {
  bool available = zenith_numa_available() != 0;
  if (init_result == ZENITH_NUMA_OK) {
    EXPECT_TRUE(available);
  }
}

// Topology tests
TEST_F(NumaBackendTest, NumNodesPositive) {
  if (init_result == ZENITH_NUMA_OK) {
    EXPECT_GT(zenith_numa_num_nodes(), 0);
  }
}

TEST_F(NumaBackendTest, NumCpusPositive) {
  if (init_result == ZENITH_NUMA_OK) {
    EXPECT_GT(zenith_numa_num_cpus(), 0);
  }
}

TEST_F(NumaBackendTest, NodeOfCpuValid) {
  if (init_result == ZENITH_NUMA_OK) {
    int num_cpus = zenith_numa_num_cpus();
    for (int i = 0; i < num_cpus && i < 4; i++) {
      int node = zenith_numa_node_of_cpu(i);
      EXPECT_GE(node, 0);
      EXPECT_LT(node, zenith_numa_num_nodes());
    }
  }
}

// Memory allocation tests
TEST_F(NumaBackendTest, AllocOnNodeSucceeds) {
  if (init_result == ZENITH_NUMA_OK) {
    void *ptr = zenith_numa_alloc_onnode(4096, 0);
    EXPECT_NE(ptr, nullptr);
    if (ptr) {
      zenith_numa_free(ptr, 4096);
    }
  }
}

TEST_F(NumaBackendTest, AllocInterleavedSucceeds) {
  if (init_result == ZENITH_NUMA_OK) {
    void *ptr = zenith_numa_alloc_interleaved(4096);
    EXPECT_NE(ptr, nullptr);
    if (ptr) {
      zenith_numa_free(ptr, 4096);
    }
  }
}

TEST_F(NumaBackendTest, AllocLocalSucceeds) {
  if (init_result == ZENITH_NUMA_OK) {
    void *ptr = zenith_numa_alloc_local(4096);
    EXPECT_NE(ptr, nullptr);
    if (ptr) {
      zenith_numa_free(ptr, 4096);
    }
  }
}

TEST_F(NumaBackendTest, AllocOnInvalidNodeFails) {
  if (init_result == ZENITH_NUMA_OK) {
    void *ptr = zenith_numa_alloc_onnode(4096, 999);
    EXPECT_EQ(ptr, nullptr);
  }
}

// Thread binding tests
TEST_F(NumaBackendTest, BindToNodeSucceeds) {
  if (init_result == ZENITH_NUMA_OK) {
    int result = zenith_numa_bind_thread_to_node(0);
    EXPECT_EQ(result, ZENITH_NUMA_OK);

    // Unbind
    zenith_numa_unbind_thread();
  }
}

TEST_F(NumaBackendTest, BindToInvalidNodeFails) {
  if (init_result == ZENITH_NUMA_OK) {
    int result = zenith_numa_bind_thread_to_node(999);
    EXPECT_EQ(result, ZENITH_NUMA_ERR_INVALID_NODE);
  }
}

// Node info tests
TEST_F(NumaBackendTest, GetNodeInfoSucceeds) {
  if (init_result == ZENITH_NUMA_OK) {
    ZenithNumaNodeInfo info = {};
    int result = zenith_numa_get_node_info(0, &info);
    EXPECT_EQ(result, ZENITH_NUMA_OK);
    EXPECT_EQ(info.node_id, 0);
    EXPECT_GT(info.total_memory, 0ULL);
    EXPECT_GE(info.num_cpus, 0);
  }
}

TEST_F(NumaBackendTest, GetNodeInfoNullFails) {
  if (init_result == ZENITH_NUMA_OK) {
    int result = zenith_numa_get_node_info(0, nullptr);
    EXPECT_EQ(result, ZENITH_NUMA_ERR_NULL_PTR);
  }
}

// Distance tests
TEST_F(NumaBackendTest, DistanceToSelfIsMinimal) {
  if (init_result == ZENITH_NUMA_OK) {
    int dist = zenith_numa_distance(0, 0);
    EXPECT_GE(dist, 0);
    EXPECT_LE(dist, 10); // Local distance is typically 10
  }
}

int main(int argc, char **argv) {
  ::testing::InitGoogleTest(&argc, argv);
  return RUN_ALL_TESTS();
}
