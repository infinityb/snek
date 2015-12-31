extern crate rand;

use rand::Rng;
use rand::distributions::range::{Range, SampleRange};
use rand::distributions::IndependentSample;

use std::collections::{
    BTreeMap,
    btree_map,
    VecDeque,
    vec_deque,
};

pub enum GameObject {
    Food,
    Wall,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
struct Area {
    x_off: usize,
    y_off: usize,
    width: usize,
    height: usize,
}

impl Area {
    #[inline]
    fn contains(&self, pos: &Position) -> bool {
        let x_end = self.x_off + self.width;
        let y_end = self.y_off + self.height;

        self.x_off <= pos.0 && pos.0 < x_end &&
        self.y_off <= pos.1 && pos.1 < y_end
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
struct Position(usize, usize);

impl Position {
    fn to_tuple(&self) -> (usize, usize) {
        (self.0, self.1)
    }

    fn adjacent(&self, dir: Direction) -> Option<Position> {
        match dir {
            Direction::North => {
                self.1.checked_sub(1).map(|v| Position(self.0, v))
            },
            Direction::South => {
                self.1.checked_add(1).map(|v| Position(self.0, v))
            },
            Direction::West => {
                self.0.checked_sub(1).map(|v| Position(v, self.1))
            },
            Direction::East => {
                self.0.checked_add(1).map(|v| Position(v, self.1))
            }
        }
    }
}

pub struct Snake {
    head_pos: Position,
    // from head to tail
    body: VecDeque<Direction>,
}

impl Snake {
    /// The direction from which the head came from.
    /// i.e., the neck is Direction relative to the Head
    /// The player will not be able to cause the snake to move in this direction
    pub fn neck_direction(&self) -> Direction {
        self.body.iter().nth(0).unwrap().clone()
    }

    fn grow(&mut self, dir: Direction) -> Result<Position, ()> {
        let next_pos = try!(self.head_pos
            .adjacent(dir)
            .ok_or(()));

        self.body.push_front(dir);
        self.head_pos = next_pos;

        Ok(next_pos)
    }

    pub fn shrink(&mut self) -> Result<(), ()> {
        self.body.pop_back().ok_or(()).map(|_| ())
    }

    pub fn positions(&self) -> SnakePositions {
        SnakePositions {
            head: self.head_pos,
            directions: self.body.iter(),
        }
    }
}

#[derive(Clone)]
pub struct SnakePositions<'a> {
    head: Position,
    directions: vec_deque::Iter<'a, Direction>,
}

impl<'a> Iterator for SnakePositions<'a> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<(usize, usize)> {
        self.directions.next().map(|&dir| {
            let cur_head = self.head;
            self.head = self.head.adjacent(dir.negate()).unwrap();
            cur_head.to_tuple()
        })
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Direction {
    North,
    South,
    West,
    East,
}

impl Direction {
    #[inline]
    fn negate(&self) -> Direction {
        match *self {
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
            Direction::East => Direction::West,
        }
    }

    pub fn is_north(&self) -> bool {
        match *self {
            Direction::North => true,
            Direction::South => false,
            Direction::West => false,
            Direction::East => false,
        }
    }
}


pub struct GameState {
    arena_size: Area,
    snake: Snake,
    objects: BTreeMap<Position, GameObject>,
    player_direction: Direction,
    force_grow: bool,
}

impl GameState {
    pub fn new(width: usize, height: usize) -> GameState {
        let mut snake_body = VecDeque::new();
        snake_body.push_front(Direction::West);

        let arena_size = Area {
            x_off: 0,
            y_off: 0,
            width: width,
            height: height,
        };

        let start_x = arena_size.x_off + arena_size.width / 4;
        let start_y = arena_size.y_off + arena_size.height / 2;

        let mut objects = BTreeMap::new();
        GameState {
            arena_size: arena_size,
            snake: Snake {
                head_pos: Position(start_x, start_y),
                body: snake_body,
            },
            objects: objects,
            player_direction: Direction::East,
            force_grow: false,
        }
    }

    pub fn set_user_direction(&mut self, direction: Direction) {
        if self.snake.neck_direction() != direction.negate() {
            self.player_direction = direction;
        }
    }

    pub fn set_force_grow(&mut self, grow: bool) {
        self.force_grow = grow;
    }

    /// Cause a time quantum to pass.  Panicks if our snake is zero-sized.
    pub fn tick(&mut self) -> Result<(), GameOver> {
        assert!(!self.snake.body.is_empty());

        let next_pos = try!(self.snake.grow(self.player_direction)
            .map_err(|()| GameOver::Died));

        if !self.arena_size.contains(&next_pos) {
            return Err(GameOver::Died);
        }

        // determine if we hit ourself
        for pos in self.snake.positions().skip(1) {
            if next_pos.to_tuple() == pos {
                return Err(GameOver::Died);
            }
        }

        let mut hit_food = self.force_grow;
        // determine if we will hit any food
        if let Some(obj) = self.objects.remove(&next_pos) {
            match obj {
                GameObject::Food => hit_food = true,
                GameObject::Wall => return Err(GameOver::Died),
            }
        }

        while self.objects.len() < 1 {
            let x_range = Range::new(0, self.arena_size.width);
            let y_range = Range::new(0, self.arena_size.height);

            let mut rng = rand::thread_rng();
            let x = self.arena_size.x_off + x_range.ind_sample(&mut rng);
            let y = self.arena_size.y_off + y_range.ind_sample(&mut rng);
            let food_pos = (x, y);

            for pos in self.snake.positions() {
                if pos == food_pos {
                    continue;
                }
            }

            self.objects.insert(Position(x, y), GameObject::Food);
        }

        if !hit_food {
            // we must be of positive length afterwards if our initial length was positive.
            self.snake.shrink().unwrap();
        }

        Ok(())
    }

    pub fn get_snake(&self) -> &Snake {
        &self.snake
    }

    pub fn object_iter(&self) -> ObjectIter {
        ObjectIter {
            objects: self.objects.iter(),
        }
    }
}

#[derive(Clone)]
pub struct ObjectIter<'a> {
    objects: btree_map::Iter<'a, Position, GameObject>,
}

impl<'a> Iterator for ObjectIter<'a> {
    type Item = ((usize, usize), &'a GameObject);

    fn next(&mut self) -> Option<((usize, usize), &'a GameObject)> {
        self.objects.next().map(|(pos, dir)| {
            (pos.to_tuple(), dir)
        })
    }
}

#[derive(Debug)]
pub enum GameOver {
    Died,
}

#[test]
fn it_works() {
    let mut state = GameState::new(64, 64);
}

#[test]
fn snake_iteration() {
    let mut snake_body = VecDeque::new();
    snake_body.push_front(Direction::West);

    let mut snake = Snake {
        head_pos: Position(1024, 1024),
        body: snake_body,
    };
    snake.grow(Direction::West).unwrap();
    snake.grow(Direction::West).unwrap();

    let mut positions = snake.positions();
    assert_eq!(positions.next().unwrap(), (1022, 1024));
    assert_eq!(positions.next().unwrap(), (1023, 1024));
    assert_eq!(positions.next().unwrap(), (1024, 1024));
    assert!(positions.next().is_none());
}
